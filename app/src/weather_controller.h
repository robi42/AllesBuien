#pragma once

#include <QtQml/QQmlApplicationEngine>

class WeatherController;

void
setWeatherController(QQmlApplicationEngine& engine,
                     WeatherController& controller);
