#pragma once

#include <QtQml/QQmlApplicationEngine>

class ThemeController;

void setThemeController(
  QQmlApplicationEngine& engine,
  ThemeController& controller);
