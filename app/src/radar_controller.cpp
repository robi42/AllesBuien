#include "radar_controller.h"

#include <QtQml/QQmlContext>

#include "allesbuien/src/radar_controller.cxxqt.h"

void
setRadarController(QQmlApplicationEngine& engine, RadarController& controller)
{
  engine.rootContext()->setContextProperty("radarController", &controller);
}
