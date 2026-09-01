#pragma once

#include <QtQml/QQmlApplicationEngine>

class RadarController;

void
setRadarController(QQmlApplicationEngine& engine, RadarController& controller);
