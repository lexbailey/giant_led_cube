# Giant LED Rubik's cube

It's a giant LED Rubik's cube!

Designed and built by some folks from York Hackspace as an installation for EMF Camp 2022.

## Hardware

The parts of the cube are:
 - a raspberry pi pico
   - the pico talks to some ws281x LEDs
   - a bunch of paddle switches on the edges of the cube for inputting twists
 - a raspberry pi 4
   - which is connected to the pico, and runs all the non-hardware bits of the software
   - speakers driven by the pi 4
 - a PC near the cube, connected to the same network as the cube (that network being the internet)
   - the PC has a GUI on it, which allows players to scramble the cube and start a time (among other things)


## Software

the software is in several parts:

device: this is the software that runs on the raspberry pi pico. it controls the leds and reads the switches
service: this runs on the pi4 inside the cube and does the configuration, timing, etc. for development purposes, this can also run on any sensible laptop
controller: this is the gui part, which the player sees

## Building

This is all rust (nearly), so you can build and run things with cargo

### The service

   cargo run -- <name_of_config_file> <name_of_cube_serial_device> --tcp localhost:9876 

the service will open a tcp socket for the controller to connect to

the cube serial device needs to exist, might be handy to have a development mode where it doesn't, but currently this mode doesn't exist

the config file needn't exist at first, it will be created if any config is changed by the controller

the config file _should_ define a secret, this is currently a TODO and the secret is hard coded

### The controller

There's a couple of modes for this, an OpenGL gui and a CLI.

The opengl gui is a WIP, the CLI is also a WIP but is much less of a WIP.

To run the CLI:

    cargo run --bin cube_control_cli --features=cli

To run the OpenGL version:

    cargo run --bin cube_control_opengl --features="opengl"

(as of time of writing the OpenGL version doesn't even compile)

### The device

The device code is part rust and part C++ (well, more like C, but actually C++ because of that one library that was C++ so I kinda had to)

This is built with cmake, because C++ ... I don't like it, but it's very much what the raspi pico community likes

To do the build for the device code:

    cd device_src
    mkdir build
    cd build
    cmake -GNinja ..
    ninja

This will create an elf file and a uf2 file

## Wiring

The device wiring is designed so that mostly you don't have to care about it

The LED pin should be obvious from the code.

The switch inputs only depend on the full list of input pins being correct. The exact order doesn't mater, again see the source code in main.cpp

## Running

To actually run the cube, you need the cube, the service, and the controller.

Connect the cube device to the machine running the service, tell the service what the cube device is called.

Run the controller on any machine that can connect to the TCP socket that the service has opened. Both the service and the controller should explain everything you need in the help text (--help)

## Questions

Open an issue to ask a question, be nice, be friendly, and you'll get a nice friendly reply back :)
