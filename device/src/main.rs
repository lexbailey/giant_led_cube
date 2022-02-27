#![no_std]
#![no_main]


// These imports are _very_ messy. TODO: tidy up some things, make some things more specific, eliminate * imports
use cortex_m_rt::entry;

use defmt::*;
use defmt_rtt as _;

use embedded_hal::digital::v2::OutputPin;

use embedded_time::rate::*;

use embedded_hal::spi::MODE_0;
use embedded_time::duration::Milliseconds;
use embedded_time::fixed_point::FixedPoint;
use embedded_time::rate::Extensions;

use panic_halt as _;

use rp_pico::hal::prelude::*;
use rp_pico::hal::pac;
use rp_pico::hal;

use rp_pico::hal::{gpio::FunctionSpi, sio::Sio, spi::Spi};
use smart_leds::{SmartLedsWrite, RGB};
use ws2812_spi::Ws2812;

const SYS_HZ: u32 = 125_000_000_u32;

#[entry]
fn main() -> ! {
    info!("Program start");

    const DELAY: Milliseconds<u32> = Milliseconds::<u32>(40);
    const NUM_LEDS: usize = 100;

    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = hal::watchdog::Watchdog::new(pac.WATCHDOG);

    // Configure the clocks
    //
    // Our default is 12 MHz crystal input, 125 MHz system clock
    let clocks = hal::clocks::init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().integer());

    let sio = Sio::new(pac.SIO);

    let pins = rp_pico::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // These are implicitly used by the spi driver if they are in the correct mode
    let _spi_sclk = pins.gpio6.into_mode::<FunctionSpi>();
    let _spi_mosi = pins.gpio7.into_mode::<FunctionSpi>();
    let _spi_miso = pins.gpio4.into_mode::<FunctionSpi>();
    let spi = Spi::<_, _, 8>::new(pac.SPI0).init(
        &mut pac.RESETS,
        SYS_HZ.Hz(),
        3_000_000u32.Hz(),
        &MODE_0,
    );

    let mut ws = Ws2812::new(spi);

    let mut data: [RGB<u8>; NUM_LEDS] = [RGB::default(); NUM_LEDS];

    let mut led_pin = pins.led.into_push_pull_output();

    let mut n = 0usize;
    let mut d = 1isize;

    // TODO render an actual cube, not just a placeholder pattern
    loop {
        n=(n as isize + d) as usize;
        if n >= 50{
            d = -1;
        }
        if n <= 0{
            d = 1;
        }

        for (i, led) in data.iter_mut().enumerate() {
            led.r = if i < n { 0xff } else { 0x00 };
            led.g = if i < n { 0xff } else { 0x00 };
            led.b = if i < n { 0xff } else { 0x00 };
        }
        
        ws.write(data.iter().cloned()).unwrap();
        delay.delay_ms(DELAY.integer());
    }

}
