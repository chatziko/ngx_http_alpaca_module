cmake_minimum_required(VERSION 3.0)
project(LibMap C)
add_library(map STATIC map.c)
install(TARGETS map DESTINATION .)
