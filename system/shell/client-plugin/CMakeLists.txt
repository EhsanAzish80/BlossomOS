cmake_minimum_required(VERSION 3.24)
project(blossom_shell_client_plugin LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_AUTOMOC ON)
set(QML_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/qml")

find_package(Qt6 6.4 REQUIRED COMPONENTS Core DBus Qml)

qt_add_qml_module(blossom-shell-client-plugin
    URI Blossom.Shell
    VERSION 1.0
    OUTPUT_DIRECTORY "${QML_OUTPUT_DIRECTORY}/Blossom/Shell"
    SOURCES
        blossombroker.cpp
        blossombroker.h
)

target_link_libraries(blossom-shell-client-plugin PRIVATE Qt6::Core Qt6::DBus Qt6::Qml)
target_compile_options(blossom-shell-client-plugin PRIVATE -Wall -Wextra -Wpedantic -Werror)

option(BLOSSOM_BUILD_CLIENT_BUS_TEST "Build isolated real-service client test" OFF)
if(BLOSSOM_BUILD_CLIENT_BUS_TEST)
    add_executable(blossom-client-bus-test ../tests/client_bus.cpp)
    target_include_directories(blossom-client-bus-test PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}")
    target_link_libraries(blossom-client-bus-test PRIVATE blossom-shell-client-plugin Qt6::Core Qt6::DBus Qt6::Qml)
endif()

# Ship both the QML loader and its backing library in the root-owned module
# directory. Installed artifacts must not depend on the developer build tree.
set_target_properties(blossom-shell-client-plugin blossom-shell-client-pluginplugin PROPERTIES
    LIBRARY_OUTPUT_DIRECTORY "${QML_OUTPUT_DIRECTORY}/Blossom/Shell"
    BUILD_WITH_INSTALL_RPATH TRUE
    INSTALL_RPATH "$ORIGIN"
)
