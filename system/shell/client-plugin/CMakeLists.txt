cmake_minimum_required(VERSION 3.24)
project(blossom_shell_client_plugin LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_AUTOMOC ON)

find_package(Qt6 6.5 REQUIRED COMPONENTS Core DBus Qml)

qt_add_qml_module(blossom-shell-client-plugin
    URI Blossom.Shell
    VERSION 1.0
    SOURCES
        blossombroker.cpp
        blossombroker.h
)

target_link_libraries(blossom-shell-client-plugin PRIVATE Qt6::Core Qt6::DBus Qt6::Qml)
target_compile_options(blossom-shell-client-plugin PRIVATE -Wall -Wextra -Wpedantic -Werror)
