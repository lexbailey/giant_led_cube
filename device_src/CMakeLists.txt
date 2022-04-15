cmake_minimum_required(VERSION 3.13)

add_custom_command(
    OUTPUT ../target/thumbv6m-none-eabi/debug/libdevice.a
    DEPENDS ../../device/src/lib.rs
    COMMAND cd ../../device && cargo build --profile dev
)

add_custom_target(LibDevice
    DEPENDS ../target/thumbv6m-none-eabi/debug/libdevice.a
)
 
include(pico_sdk_import.cmake)
link_directories(../target/thumbv6m-none-eabi/debug/)

set(PICO_CXX_ENABLE_RTTI 1)
set(PICO_CXX_ENABLE_EXCEPTIONS 1)

project(CubeDevice C CXX ASM)

add_executable(CubeDevice main.cpp)
 
target_link_libraries(CubeDevice pico_stdlib hardware_pwm PicoLed device)

pico_sdk_init()

include(PicoLED/PicoLed.cmake)

add_dependencies(CubeDevice LibDevice)

set_property(SOURCE main.cpp APPEND PROPERTY OBJECT_DEPENDS ../target/thumbv6m-none-eabi/debug/libdevice.a)
set_property(SOURCE main.cpp APPEND PROPERTY OBJECT_DEPENDS ../../device/src/lib.rs)

# Enable USB, UART output
pico_enable_stdio_usb(CubeDevice 1)
pico_enable_stdio_uart(CubeDevice 0)
 
# Enable extra outputs (SWD?)
pico_add_extra_outputs(CubeDevice)
