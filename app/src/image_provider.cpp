#include "image_provider.h"
#include "image_provider.cxxqt.h"

QQmlImageProviderBase*
imageProviderNew(rust::cxxqtlib1::QQmlImageProviderBaseImageType image_type)
{
  return new ImageProvider(image_type);
}
