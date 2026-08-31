#include "theme_controller.h"
#include <QtQml/QQmlContext>
#include "allesbuien/src/theme_controller.cxxqt.h"
void setThemeController(QQmlApplicationEngine& engine, ThemeController& controller) { engine.rootContext()->setContextProperty("themeController", &controller); }
