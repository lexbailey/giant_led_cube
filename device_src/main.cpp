#include <stdio.h>
#include <stdint.h>
#include <math.h>
#include "pico/stdlib.h"
#include <PicoLed.hpp>
typedef void Cube;
typedef void OutputMap5Faces;
extern "C" {
#include "../device/target/include/cube_data.h"
#include "../device/target/include/model_size_info.h"
}

const uint LED_PIN = PICO_DEFAULT_LED_PIN;
const uint LED_STRIP_PIN = 7;
const uint LED_STRIP_LEN = 45+1; // five faces of 9 LEDs, plus the initial skipped LED

uint8_t thecube[CUBE_STRUCT_BYTES];
uint8_t frames[3][CUBE_STRUCT_BYTES];
uint8_t mapping[OUTPUT_ARRAY_BYTES];
uint32_t led_data[45];

#define NUM_SKIP (1)

#define MODE_PLAY (0)
#define MODE_CONFIG (1)
#define MODE_UPDATE_READ (2)
#define MODE_LEDMAP_READ (3)
#define MODE_SWITCHMAP_READ (4)
#define MODE_BRIGHTNESS (5)

#define NUM_SUBFACES (6*9)
#define NUM_SWITCH_INPUTS (18)
#define MAX_INPUT_NUM (21)
#define NUM_TWISTS (18)

const int switch_inputs[NUM_SWITCH_INPUTS] = {
    2,3,4,5,6,8
    ,10,11,12,13,14,15
    ,16,17,18,19,20,21
};

#define FACES_BUFLEN ((NUM_SUBFACES*2)+1)
#define INPUTS_BUFLEN ((NUM_TWISTS*2)+1)

#if FACES_BUFLEN > INPUTS_BUFLEN
#define BUFLEN FACES_BUFLEN
#else
#define BUFLEN INPUTS_BUFLEN
#endif
volatile char update_buffer[BUFLEN]; // big enough for two chars per subface, or for all the inputs

int cur_frame = 0;
absolute_time_t frame_time;

void update_leds(PicoLed::PicoLedController ledStrip){
    absolute_time_t now = get_absolute_time();
    if (absolute_time_diff_us(frame_time, now) > 50000){
        cur_frame += 1;
        if (cur_frame > 3) {cur_frame = 3;}
        frame_time = now;
    }
    uint8_t *frame = cur_frame >= 3? thecube : frames[cur_frame];
    get_data(frame, mapping, led_data);
    //printf("?%d %d %d %d\n;", led_data[0], led_data[1], led_data[2], led_data[3]);
    for (int i = NUM_SKIP; i <= 45; i++){
        int led = i - NUM_SKIP;
        ledStrip.setPixelColor(i, PicoLed::RGB((led_data[led]>>16) & 0xff, (led_data[led] >> 8) & 0xff, led_data[led] & 0xff));
    }
    ledStrip.show();
    sleep_us(400);
}

const char* blank = "f ";

uint8_t brightness = 40; // start on low brightness

int switch_to_twist_id[MAX_INPUT_NUM+1];
int twist_id_to_switch[18];
const char* switch_map[MAX_INPUT_NUM+1];
int pending_twist[MAX_INPUT_NUM+1];
absolute_time_t pending_twist_time[MAX_INPUT_NUM+1];

absolute_time_t switch_last_pressed[MAX_INPUT_NUM+1];
absolute_time_t switch_last_released[MAX_INPUT_NUM+1];
int switch_blocked[MAX_INPUT_NUM+1];

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

int invert_twist(int gpio){
    // The order of the twist names is such that the last bit indicates inverse
    // flip the last bit to invert a twist
    return twist_id_to_switch[switch_to_twist_id[gpio] ^ 1];
}

void log_switch_pressed(int gpio, absolute_time_t time){
    switch_blocked[gpio] = 1;
    switch_last_pressed[gpio] = time;
}

void log_switch_released(int gpio, absolute_time_t time){
    switch_blocked[gpio] = 0;
    switch_last_released[gpio] = time;
}

int can_twist(int gpio, absolute_time_t time, int skip_inverse){
    int inverse = invert_twist(gpio);
    if (switch_blocked[gpio] || (!skip_inverse && switch_blocked[inverse])){
        return 0;
    }
    int64_t d1 = absolute_time_diff_us(switch_last_pressed[gpio], time);
    int64_t d2 = absolute_time_diff_us(switch_last_released[inverse], time);
    return (d1 > 150000) && (skip_inverse || (d2 > 500000));
}

int mode = MODE_PLAY;

void do_twist(int gpio){
    const char* twist = switch_map[gpio];
    if (mode == MODE_PLAY) {
        printf("*%s;\n", twist);
        twist_cube(thecube, (uint8_t*)twist, 2, frames[0], frames[1], frames[2]);
        cur_frame = 0;
        if (is_solved(thecube)) {
            printf("#\n");
        }
    }
    if (mode == MODE_CONFIG) {
        printf("i%d;\n", gpio);
    }
}

void switch_isr(uint gpio, uint32_t events){
    absolute_time_t now = get_absolute_time();
    //printf("?%d,0x%02x\n;",gpio,events);
    if (events & 4){
        if (can_twist(gpio, now, mode == MODE_CONFIG)){
            pending_twist[gpio] = 1;
            pending_twist_time[gpio] = now;
        }
        log_switch_pressed(gpio, now);
    }
    if (events & 8){
        log_switch_released(gpio, now);
    }
    gpio_acknowledge_irq(gpio, events);
}

void check_twists(){
    absolute_time_t now = get_absolute_time();
    for (int i = 0; i <= NUM_SWITCH_INPUTS-1; i++){
        int gpio = switch_inputs[i];
        if (pending_twist[gpio]){
            int64_t d = absolute_time_diff_us(pending_twist_time[gpio], now);
            if (d > 5000){
                pending_twist[gpio] = 0;
                if (gpio_get(gpio) == 0){
                    do_twist(gpio);
                }
            }
        }
    }
    
}

int main(){
    stdio_init_all();
    // LED flash is to make it obvious when the pico boots
    gpio_init(LED_PIN);
    gpio_set_dir(LED_PIN, GPIO_OUT);
    gpio_put(LED_PIN, 1);
    for (int i = 0; i<= 5; i++){
        sleep_ms(50);
        gpio_put(LED_PIN, 0);
        sleep_ms(50);
        gpio_put(LED_PIN, 1);
    }
    gpio_set_slew_rate(LED_PIN, GPIO_SLEW_RATE_FAST);
    //gpio_set_drive_strength(LED_PIN, GPIO_DRIVE_STRENGTH_12MA);

    absolute_time_t t = get_absolute_time();

    for (int i = 0; i <= NUM_SWITCH_INPUTS-1; i++){
        int pin = switch_inputs[i];
        switch_map[pin] = blank;
        switch_to_twist_id[pin] = 0;
        switch_last_pressed[pin] = t;
        switch_last_released[pin] = t;
        switch_blocked[pin] = 0;
        pending_twist[pin] = 0;
        pending_twist_time[pin] = t;
        gpio_init(pin);
        gpio_set_dir(pin, GPIO_IN);
        gpio_pull_up(pin);
        gpio_set_irq_enabled_with_callback(pin, 12, true, switch_isr);
    }

    init_cube(thecube, mapping);

    auto ledStrip = PicoLed::addLeds<PicoLed::WS2812B>(pio0, 0, LED_STRIP_PIN, LED_STRIP_LEN, PicoLed::FORMAT_RGB);
    ledStrip.setBrightness(brightness);
    ledStrip.setPixelColor(0, PicoLed::RGB(0,0,0));

    int next_mode = MODE_PLAY;
    int update_pos = 0;

    while(1){
        int ic = getchar_timeout_us(0);
        if (ic != PICO_ERROR_TIMEOUT){
            char c = (char) ic & 0xff;
            if (mode == MODE_BRIGHTNESS) {
                brightness = c;
                ledStrip.setBrightness(brightness);
                mode = next_mode;
            }
            else if (c == 'c'){ // CONFIG mode
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
            else if (c == '%'){ // brightness control
                next_mode = mode;
                mode = MODE_BRIGHTNESS;
            }
            else {
                if (mode == MODE_UPDATE_READ){
                    if ((update_pos) >= BUFLEN) { printf("?badstateupdate\n;"); }
                    else{
                        update_buffer[update_pos++] = c;
                        if (update_pos >= NUM_SUBFACES) {
                            mode = next_mode;
                            update_buffer[update_pos] = '\0';
                            update_from_string(thecube, (uint8_t *)update_buffer);
                        }
                    }
                }
                if (mode == MODE_LEDMAP_READ){
                    if ((update_pos) >= BUFLEN) { printf("?badstateledmap\n;"); }
                    else{
                        update_buffer[update_pos++] = c-48;
                        if (update_pos >= 90) {
                            mode = next_mode;
                            remap_outputs(mapping, (uint8_t *)update_buffer);
                        }
                    }
                }
                if (mode == MODE_SWITCHMAP_READ){
                    if ((update_pos) >= BUFLEN) { printf("?badstateswitchmap\n;"); }
                    else{
                        update_buffer[update_pos++] = c;
                        if (update_pos >= (18*2)) {
                            mode = next_mode;
                            for (int i = 0; i<= 18-1; i++){
                                int p = i*2;
                                int switch_num = ((update_buffer[p]-48)*10) + (update_buffer[p+1]-48);
                                if (switch_num > MAX_INPUT_NUM){
                                    printf("?numtoohigh\n;"); //wut?
                                }
                                else {
                                    switch_to_twist_id[switch_num] = i;
                                    twist_id_to_switch[i] = switch_num;
                                    switch_map[switch_num] = twists[i];
                                }
                            }
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
        check_twists();
        update_leds(ledStrip);
    }
}
