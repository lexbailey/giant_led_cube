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
#define MODE_LEDMAP_READ (3)
#define MODE_SWITCHMAP_READ (4)

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

#define NUM_SWITCH_INPUTS (20)
const int switch_inputs[NUM_SWITCH_INPUTS] = {
    2,3,4,5
    ,6,22,8,9
    ,10,11,12,13
    ,14,15,16,17
    ,18,19,20,21
};

const char* blank = "f ";

const char* switch_map[24];
/*
 = {
"  "
,"  "
,"m "
,"m'"
,"f'"
,"f "
,"b "
,"  "
,"l "
,"l'"
,"d "
,"d'"
,"e "
,"e'"
,"u'"
,"u "
,"  "
,"  "
,"s'"
,"s "
,"r'"
,"r "
,"b'"
,"  "
};*/

absolute_time_t switch_timeout[24];

const char* twists[18] = {
    "f "
    ,"f'"
    ,"b "
    ,"b'"
    ,"r "
    ,"r'"
    ,"l "
    ,"l'"
    ,"u "
    ,"u'"
    ,"d "
    ,"d'"
    ,"e "
    ,"e'"
    ,"m "
    ,"m'"
    ,"s "
    ,"s'"
};

int mode = MODE_PLAY;

void switch_isr(uint gpio, uint32_t events){
    absolute_time_t timeout = switch_timeout[gpio];
    absolute_time_t now = get_absolute_time();
    //printf("%d,0x%02x\n",gpio,events);
    if (absolute_time_diff_us(now, timeout) < 0){
        switch_timeout[gpio] = make_timeout_time_ms(200);
        if (events & 0x4){
            const char* twist = switch_map[gpio];
            if (mode == MODE_CONFIG) {
                printf("i%d;\n", gpio);
            }
            if (mode == MODE_PLAY) {
                printf("*%s;\n", twist);
                twist_cube(thecube, (uint8_t*)twist, 2);
            }
        }
    }
    gpio_acknowledge_irq(gpio, events);
}

int main(){
    stdio_init_all();
    gpio_init(LED_PIN);
    gpio_set_dir(LED_PIN, GPIO_OUT);
    gpio_put(LED_PIN, 1);

    absolute_time_t t = get_absolute_time();
    for (int i = 0; i <= NUM_SWITCH_INPUTS-1; i++){
        int pin = switch_inputs[i];
        switch_map[pin] = blank;
        switch_timeout[pin] = t;
        gpio_init(pin);
        gpio_set_dir(pin, GPIO_IN);
        gpio_pull_up(pin);
        gpio_set_irq_enabled_with_callback(pin, 12, true, switch_isr);
    }

    init_cube(thecube, mapping);

    auto ledStrip = PicoLed::addLeds<PicoLed::WS2812B>(pio0, 0, LED_STRIP_PIN, LED_STRIP_LEN, PicoLed::FORMAT_GRB);
    ledStrip.setBrightness(64);
    ledStrip.setPixelColor(0, PicoLed::RGB(0,0,0));

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
            else if (c == 'm'){ // new led Mapping
                next_mode = mode;
                mode = MODE_LEDMAP_READ;
                update_pos = 0;
            }
            else if (c == 'a'){ // new switch mapping
                next_mode = mode;
                mode = MODE_SWITCHMAP_READ;
                update_pos = 0;
            }
            else {
                if (mode == MODE_UPDATE_READ){
                    update_buffer[update_pos++] = c;
                    if (update_pos >= NUM_SUBFACES) {
                        mode = next_mode;
                        update_from_string(thecube, (uint8_t *)update_buffer);
                    }
                }
                if (mode == MODE_LEDMAP_READ){
                    update_buffer[update_pos++] = c-48;
                    if (update_pos >= 90) {
                        mode = next_mode;
                        remap_outputs(mapping, (uint8_t *)update_buffer);
                    }
                }
                if (mode == MODE_SWITCHMAP_READ){
                    update_buffer[update_pos++] = c;
                    if (update_pos >= (18*2)) {
                        mode = next_mode;
                        for (int i = 0; i<= 18-1; i++){
                            int p = i*2;
                            int switch_num = ((update_buffer[p]-48)*10) + update_buffer[p+1];
                            switch_map[switch_num] = twists[i];
                        }
                    }
                }
            }
        }
        
        if (mode == MODE_PLAY){
            //twist(thecube, (uint8_t *)"F", 1);
            //sleep_ms(100);
        }
        if (mode == MODE_CONFIG){
            // do nothing
        }
        update_leds(ledStrip);
    }
}
