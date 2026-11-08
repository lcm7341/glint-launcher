cmake_minimum_required(VERSION 3.20)
project(GlintLoader CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreaded")

add_library(glint_loader SHARED loader.cpp loader.def)
set_target_properties(glint_loader PROPERTIES
  OUTPUT_NAME "XInput9_1_0" PREFIX "" SUFFIX ".dll"
  RUNTIME_OUTPUT_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}/out")
target_compile_definitions(glint_loader PRIVATE UNICODE _UNICODE NOMINMAX WIN32_LEAN_AND_MEAN)
# /Brepro for reproducible builds
target_compile_options(glint_loader PRIVATE /O2 /GS- /Brepro)
target_link_options(glint_loader PRIVATE /OPT:REF /OPT:ICF /DEBUG:NONE /Brepro)
