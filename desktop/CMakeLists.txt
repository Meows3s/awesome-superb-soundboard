cmake_minimum_required(VERSION 3.21)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)

# SerialPort is needed to talk to the macropad; Multimedia for audio mixing.
# Only Widgets is required until those features land, so the skeleton builds
# on a minimal Qt install.
find_package(Qt6 6.4 REQUIRED COMPONENTS Widgets SerialPort Multimedia)

add_library(soundboard-core STATIC
  src/Protocol.cpp src/Protocol.h
  src/Config.cpp src/Config.h
  src/Mixer.cpp src/Mixer.h)
target_include_directories(soundboard-core PUBLIC src)
target_link_libraries(soundboard-core PUBLIC Qt6::Core Qt6::Gui)

add_executable(soundboard-desktop WIN32
  src/main.cpp
  src/MainWindow.cpp src/MainWindow.h
  src/SerialLink.cpp src/SerialLink.h
  src/AudioEngine.cpp src/AudioEngine.h)
target_link_libraries(soundboard-desktop PRIVATE soundboard-core Qt6::Widgets Qt6::SerialPort Qt6::Multimedia)

enable_testing()
find_package(Qt6 6.4 REQUIRED COMPONENTS Test)
foreach(t protocol mixer)
  add_executable(test_${t} tests/test_${t}.cpp)
  target_link_libraries(test_${t} PRIVATE soundboard-core Qt6::Test)
  add_test(NAME ${t} COMMAND test_${t})
endforeach()

# Windows: copy the Qt DLLs/plugins next to the exe after build so it runs
# straight from the build dir.
if(WIN32)
  get_target_property(_qmake Qt6::qmake IMPORTED_LOCATION)
  get_filename_component(_qtbin "${_qmake}" DIRECTORY)
  find_program(WINDEPLOYQT windeployqt HINTS "${_qtbin}")
  if(WINDEPLOYQT)
    add_custom_command(TARGET soundboard-desktop POST_BUILD
      COMMAND ${WINDEPLOYQT} --no-translations $<TARGET_FILE:soundboard-desktop>
      COMMENT "windeployqt")
  endif()
endif()
