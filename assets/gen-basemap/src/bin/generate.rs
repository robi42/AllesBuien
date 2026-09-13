use std::{
    error::Error,
    fs::{self, File},
    io::{self, Write as _},
    path::{Path, PathBuf},
    time::Duration,
};

use gen_basemap::{
    BOUNDARY, HEIGHT, LAND, MapProjection, WATER, WIDTH,
    geojson::{GeoPoint, Polygon, parse_lines, parse_polygons},
    raster::{draw_line, fill_polygon},
    repository_root, sha256_hex,
};
use image::{Rgb, RgbImage};

const NATURAL_EARTH_SHA: &str = "ca96624a56bd078437bca8184e78163e5039ad19";
const CACHE_DIR: &str = "/tmp/gen-basemap-cache";
const LAND_SOURCE: NaturalEarthFile = NaturalEarthFile {
    name: "ne_10m_land.geojson",
    url: "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/ca96624a56bd078437bca8184e78163e5039ad19/geojson/ne_10m_land.geojson",
};
const LAKES_SOURCE: NaturalEarthFile = NaturalEarthFile {
    name: "ne_10m_lakes.geojson",
    url: "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/ca96624a56bd078437bca8184e78163e5039ad19/geojson/ne_10m_lakes.geojson",
};
const BOUNDARIES_SOURCE: NaturalEarthFile = NaturalEarthFile {
    name: "ne_10m_admin_0_boundary_lines_land.geojson",
    url: "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/ca96624a56bd078437bca8184e78163e5039ad19/geojson/ne_10m_admin_0_boundary_lines_land.geojson",
};

/// One pinned Natural Earth download: its cache file name and source URL.
struct NaturalEarthFile {
    name: &'static str,
    url: &'static str,
}

/// The three parsed layers the basemap is painted from, in painting order.
struct Sources {
    land: Vec<Polygon>,
    lakes: Vec<Polygon>,
    boundaries: Vec<Vec<GeoPoint>>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let keep_intermediate = parse_arguments()?;
    let root = repository_root()?;
    let cache = Path::new(CACHE_DIR).join(NATURAL_EARTH_SHA);
    fs::create_dir_all(&cache)?;
    let sources = fetch_sources(&download_agent(), &cache)?;

    let output_dir = root.join("app/assets");
    fs::create_dir_all(&output_dir)?;
    let intermediate = keep_intermediate.then(|| output_dir.join("basemap-nl-pre-lakes.png"));
    let image = render_basemap(&sources, intermediate.as_deref())?;
    publish(&root, &output_dir, &image)
}

/// An HTTPS-only client with one generous timeout for the largest download.
fn download_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(120)))
        .https_only(true)
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            "/",
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .into()
}

/// Downloads any source missing from `cache`, then parses all three layers.
///
/// Every download finishes before the first parse, so a network failure is
/// reported before any time is spent decoding the files already present.
fn fetch_sources(agent: &ureq::Agent, cache: &Path) -> Result<Sources, Box<dyn Error>> {
    let land = ensure_cached(agent, cache, &LAND_SOURCE)?;
    let lakes = ensure_cached(agent, cache, &LAKES_SOURCE)?;
    let boundaries = ensure_cached(agent, cache, &BOUNDARIES_SOURCE)?;
    Ok(Sources {
        land: parse_polygons(&fs::read(land)?)?,
        lakes: parse_polygons(&fs::read(lakes)?)?,
        boundaries: parse_lines(&fs::read(boundaries)?)?,
    })
}

/// Paints land over water, then lakes back to water, then the land boundaries.
///
/// When `intermediate` names a path, the canvas is saved there after the land
/// pass and before the lakes, which is the stage a lake-fill regression shows.
fn render_basemap(
    sources: &Sources,
    intermediate: Option<&Path>,
) -> Result<RgbImage, image::ImageError> {
    let projection = MapProjection::default();
    let mut image = RgbImage::from_pixel(WIDTH, HEIGHT, Rgb(WATER));
    for polygon in &sources.land {
        fill_polygon(&mut image, &projection, polygon, Rgb(LAND));
    }
    if let Some(path) = intermediate {
        image.save(path)?;
    }
    for polygon in &sources.lakes {
        fill_polygon(&mut image, &projection, polygon, Rgb(WATER));
    }
    for line in &sources.boundaries {
        draw_line(&mut image, &projection, line, Rgb(BOUNDARY));
    }
    Ok(image)
}

/// Writes the shipped basemap and records its digest in `BASEMAP-LICENSE`.
fn publish(root: &Path, output_dir: &Path, image: &RgbImage) -> Result<(), Box<dyn Error>> {
    let output = output_dir.join("basemap-nl.png");
    image.save(&output)?;
    let digest = sha256_hex(&fs::read(&output)?);
    update_license(&root.join("assets/BASEMAP-LICENSE"), &digest)?;
    println!("generated {} ({digest})", output.display());
    Ok(())
}

fn parse_arguments() -> Result<bool, Box<dyn Error>> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [] => Ok(false),
        [flag] if flag == "--keep-intermediate" => Ok(true),
        _ => Err("usage: generate [--keep-intermediate]".into()),
    }
}

fn ensure_cached(
    agent: &ureq::Agent,
    cache: &Path,
    source: &NaturalEarthFile,
) -> Result<PathBuf, Box<dyn Error>> {
    let NaturalEarthFile { name, url } = *source;
    let destination = cache.join(name);
    if destination
        .metadata()
        .is_ok_and(|metadata| metadata.len() > 0)
    {
        println!("using cached {}", destination.display());
        return Ok(destination);
    }
    let temporary = cache.join(format!("{name}.part"));
    println!("downloading {url}");
    let mut response = agent.get(url).call()?;
    let mut reader = response.body_mut().as_reader();
    let mut file = File::create(&temporary)?;
    io::copy(&mut reader, &mut file)?;
    file.flush()?;
    fs::rename(temporary, &destination)?;
    Ok(destination)
}

fn update_license(path: &Path, digest: &str) -> Result<(), Box<dyn Error>> {
    let contents = fs::read_to_string(path)?;
    let mut updated = contents
        .lines()
        .filter(|line| !line.starts_with("Basemap SHA-256:"))
        .collect::<Vec<_>>()
        .join("\n");
    updated.push('\n');
    updated.push_str("Basemap SHA-256: ");
    updated.push_str(digest);
    updated.push('\n');
    fs::write(path, updated)?;
    Ok(())
}
