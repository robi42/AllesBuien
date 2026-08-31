#include "weather_controller.h"
#include <QtQml/QQmlContext>
#include "allesbuien/src/weather_controller.cxxqt.h"
void setWeatherController(QQmlApplicationEngine& engine, WeatherController& controller) { engine.rootContext()->setContextProperty("weatherController", &controller); }
