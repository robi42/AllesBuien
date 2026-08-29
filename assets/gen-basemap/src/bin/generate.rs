use std::{
    error::Error,
    fs::{self, File},
    io::{self, Write as _},
    path::{Path, PathBuf},
    time::Duration,
};

use gen_basemap::{
    BOUNDARY, HEIGHT, LAND, MapProjection, WATER, WIDTH,
    geojson::{parse_lines, parse_polygons},
    raster::{draw_line, fill_polygon},
    sha256_hex,
};
use image::{Rgb, RgbImage};

const NATURAL_EARTH_SHA: &str = "ca96624a56bd078437bca8184e78163e5039ad19";
const CACHE_DIR: &str = "/tmp/gen-basemap-cache";
const SOURCES: [(&str, &str); 3] = [
    (
        "ne_10m_land.geojson",
        "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/ca96624a56bd078437bca8184e78163e5039ad19/geojson/ne_10m_land.geojson",
    ),
    (
        "ne_10m_lakes.geojson",
        "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/ca96624a56bd078437bca8184e78163e5039ad19/geojson/ne_10m_lakes.geojson",
    ),
    (
        "ne_10m_admin_0_boundary_lines_land.geojson",
        "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/ca96624a56bd078437bca8184e78163e5039ad19/geojson/ne_10m_admin_0_boundary_lines_land.geojson",
    ),
];

fn main() -> Result<(), Box<dyn Error>> {
    let keep_intermediate = parse_arguments()?;
    let root = repository_root()?;
    let cache = Path::new(CACHE_DIR).join(NATURAL_EARTH_SHA);
    fs::create_dir_all(&cache)?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(120)))
        .https_only(true)
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            "/",
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .into();

    let source_paths = SOURCES
        .iter()
        .map(|(name, url)| ensure_cached(&agent, &cache, name, url))
        .collect::<Result<Vec<_>, _>>()?;
    let land = parse_polygons(&fs::read(&source_paths[0])?)?;
    let lakes = parse_polygons(&fs::read(&source_paths[1])?)?;
    let boundaries = parse_lines(&fs::read(&source_paths[2])?)?;

    let projection = MapProjection::default();
    let mut image = RgbImage::from_pixel(WIDTH, HEIGHT, Rgb(WATER));
    for polygon in &land {
        fill_polygon(&mut image, &projection, polygon, Rgb(LAND));
    }

    let output_dir = root.join("app/assets");
    fs::create_dir_all(&output_dir)?;
    if keep_intermediate {
        image.save(output_dir.join("basemap-nl-pre-lakes.png"))?;
    }
    for polygon in &lakes {
        fill_polygon(&mut image, &projection, polygon, Rgb(WATER));
    }
    for line in &boundaries {
        draw_line(&mut image, &projection, line, Rgb(BOUNDARY));
    }

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

fn repository_root() -> Result<PathBuf, io::Error> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .ok_or_else(|| io::Error::other("crate is not nested under the repository root"))
}

fn ensure_cached(
    agent: &ureq::Agent,
    cache: &Path,
    name: &str,
    url: &str,
) -> Result<PathBuf, Box<dyn Error>> {
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
    updated.push_str(&format!("\nBasemap SHA-256: {digest}\n"));
    fs::write(path, updated)?;
    Ok(())
}
