#include <stdio.h>
#include <stdint.h>
#include <math.h>
#include "pico/stdlib.h"
#include <PicoLed.hpp>
typedef void Cube;
typedef void OutputMap5Faces;
extern "C" {
#include "../device/target/include/cube_data.h"
}

const uint LED_PIN = PICO_DEFAULT_LED_PIN;
const uint LED_STRIP_PIN = 7;
const uint LED_STRIP_LEN = 45+1; // five faces of 9 LEDs, plus the initial skipped LED

uint8_t thecube[1000];
uint8_t mapping[1000];
uint32_t led_data[45];

#define MODE_PLAY (0)
#define MODE_CONFIG (1)
#define MODE_UPDATE_READ (2)
#define MODE_MAP_READ (3)

#define NUM_SUBFACES (6*9)

char inbuf[10];
volatile char update_buffer[(NUM_SUBFACES*2)+1]; // big enough for two chars per subface

void update_leds(PicoLed::PicoLedController ledStrip){
    //printf("a\r\n");
    get_data(thecube, mapping, led_data);
    for (int i = 1; i <= 45; i++){
        int led = i - 1;
        ledStrip.setPixelColor(i, PicoLed::RGB((led_data[led]>>16) & 0xff, (led_data[led] >> 8) & 0xff, led_data[led] & 0xff));
    }
    ledStrip.show();
    sleep_ms(2);
}


int main(){
    stdio_init_all();
    printf("started\n");
    gpio_init(LED_PIN);
    gpio_set_dir(LED_PIN, GPIO_OUT);
    gpio_put(LED_PIN, 1);

    init_cube(thecube, mapping);

    auto ledStrip = PicoLed::addLeds<PicoLed::WS2812B>(pio0, 0, LED_STRIP_PIN, LED_STRIP_LEN, PicoLed::FORMAT_GRB);
    ledStrip.setBrightness(64);
    ledStrip.setPixelColor(0, PicoLed::RGB(0,0,0));

    int mode = MODE_PLAY;
    int next_mode = MODE_PLAY;
    int update_pos = 0;

    while(1){
        int ic = getchar_timeout_us(0);
        if (ic != PICO_ERROR_TIMEOUT){
            char c = (char) ic & 0xff;
            if (c == 'c'){ // CONFIG mode
                mode = MODE_CONFIG;
            }
            else if (c == 'p'){ // PLAY mode
                mode = MODE_PLAY;
            }
            else if (c == 'u'){ // raw Update of display
                next_mode = mode;
                mode = MODE_UPDATE_READ;
                update_pos = 0;
            }
            else if (c == 'm'){ // new Mapping
                next_mode = mode;
                mode = MODE_MAP_READ;
                update_pos = 0;
            }
            else {
                if (mode == MODE_UPDATE_READ){
                    update_buffer[update_pos++] = c;
                    if (update_pos >= NUM_SUBFACES) {
                        mode = next_mode;
                        update_from_string(thecube, (uint8_t *)update_buffer);
                        printf("udpate\r\n");
                    }
                }
                if (mode == MODE_MAP_READ){
                    update_buffer[update_pos++] = c-48;
                    if (update_pos >= 90) {
                        mode = next_mode;
                        remap_outputs(mapping, (uint8_t *)update_buffer);
                    }
                }
            }
        }
        
        if (mode == MODE_PLAY){
            //twist(thecube, (uint8_t *)"F", 1);
            sleep_ms(100);
        }
        if (mode == MODE_CONFIG){
            // do nothing
        }
        update_leds(ledStrip);
    }
}
