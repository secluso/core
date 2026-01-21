//! Secluso IR cut filter toggler.
//!
//! SPDX-License-Identifier: GPL-3.0-or-later

use rppal::gpio::{Gpio, OutputPin};
use std::env;
use std::process;
use std::thread;
use std::time::Duration;
use anyhow::Result;
use embedded_hal::i2c::I2c;
use linux_embedded_hal::I2cdev;

// ----------------- IR/IR cut filter -----------------

const IN1_PIN: u8 = 17;      // BCM numbering
const IN2_PIN: u8 = 27;      // BCM numbering
const SLEEP_PIN: u8 = 4;     // 1=enable bridge, 0=disable bridge
const IR_PIN: u8 = 21;       // 1=disable IR (day mode), 0=enable IR (night mode)
const PULSE_MS: u64 = 120;

struct IrCut {
    in1: OutputPin,
    in2: OutputPin,
    sleep: OutputPin,
    ir: OutputPin,
}

impl IrCut {
    fn new(gpio: &Gpio, ir_on: bool) -> anyhow::Result<Self> {
        let in1 = gpio.get(IN1_PIN)?.into_output_low();
        let in2 = gpio.get(IN2_PIN)?.into_output_low();
        let sleep = gpio.get(SLEEP_PIN)?.into_output_low();

        // Initialize EN to desired final level immediately.
        let mut ir = if ir_on {
            gpio.get(IR_PIN)?.into_output_high()
        } else {
            gpio.get(IR_PIN)?.into_output_low()
        };

        // Keep the state after program exit (prevents flicker / reversion).
        ir.set_reset_on_drop(false);

        Ok(Self { in1, in2, sleep, ir })
    }

    fn night(&mut self) {
        // Enable IR first to avoid glitches.
        self.ir.set_high();

        // Enable bridge driver IC
        self.in1.set_low();
        self.in2.set_low();
        self.sleep.set_high();
        thread::sleep(Duration::from_millis(1000));

        // Pulse coil: IN1=LOW, IN2=HIGH
        self.in1.set_low();
        self.in2.set_high();
        thread::sleep(Duration::from_millis(PULSE_MS));
        self.in1.set_low();
        self.in2.set_low();

        // Disable bridge driver IC
        self.sleep.set_low();

    }

    fn day(&mut self) {
        // Enable bridge driver IC
        self.in1.set_low();
        self.in2.set_low();
        self.sleep.set_high();
        thread::sleep(Duration::from_millis(1000));

        // Pulse coil: IN1=HIGH, IN2=LOW
        self.in1.set_high();
        self.in2.set_low();
        thread::sleep(Duration::from_millis(PULSE_MS));
        self.in1.set_low();
        self.in2.set_low();

        // Disable bridge driver IC
        self.sleep.set_low();

        // Disable IR last.
        self.ir.set_low();
    }
}

// ----------------- Ambient light sensor -----------------

const AMBIENT_ADDR: u8 = 0x52;

// Register map (APDS-9306 / APDS-9306-065)
const REG_MAIN_CTRL: u8 = 0x00;      // ALS_EN is bit 1
const REG_ALS_MEAS_RATE: u8 = 0x04;  // default 0x22
const REG_ALS_GAIN: u8 = 0x05;       // default 0x01 (gain 3)
const REG_PART_ID: u8 = 0x06;        // APDS-9306-065 default 0xB3
const REG_MAIN_STATUS: u8 = 0x07;    // ALS data status bit indicates new data
const REG_ALS_DATA_0: u8 = 0x0D;     // 0x0D..0x0F = 20-bit ALS result (LSB aligned)

fn als_write_u8<I: I2c>(i2c: &mut I, reg: u8, val: u8) -> core::result::Result<(), I::Error> {
    i2c.write(AMBIENT_ADDR, &[reg, val])
}

fn als_read_u8<I: I2c>(i2c: &mut I, reg: u8) -> core::result::Result<u8, I::Error> {
    let mut b = [0u8; 1];
    i2c.write_read(AMBIENT_ADDR, &[reg], &mut b)?;
    Ok(b[0])
}

fn read_als_20bit<I: I2c>(i2c: &mut I) -> core::result::Result<u32, I::Error> {
    // Block read 3 bytes starting at 0x0D to keep bytes from the same conversion.
    let mut b = [0u8; 3];
    i2c.write_read(AMBIENT_ADDR, &[REG_ALS_DATA_0], &mut b)?;

    let raw = (b[0] as u32) | ((b[1] as u32) << 8) | (((b[2] as u32) & 0x0F) << 16);
    Ok(raw)
}

// ----------------- Temperature sensor -----------------

const TEMP_ADDR: u8 = 0x48;      // TMP1075 default address
const REG_TEMP: u8 = 0x00;   // Temperature register

fn read_temp_raw<I: I2c>(i2c: &mut I) -> core::result::Result<i16, I::Error> {
    let mut b = [0u8; 2];
    i2c.write_read(TEMP_ADDR, &[REG_TEMP], &mut b)?;

    let raw = i16::from_be_bytes(b);

    // 12-bit value in upper bits
    Ok(raw >> 4)
}

fn read_temp_c<I: I2c>(i2c: &mut I) -> core::result::Result<f32, I::Error> {
    let raw12 = read_temp_raw(i2c)?;
    Ok(raw12 as f32 / 16.0)
}

// ----------------- Main -----------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Day,
    Night,
}

const LIGHT_THRESHOLD: u32 = 40;
const REQUIRED_CONSECUTIVE: u8 = 3;

const TEMP_THRESHOLD_C: f32 = 90.0;

fn main() -> Result<()> {
    // --- Init GPIO + IR Cut ---
    let gpio = Gpio::new()?;

    // Start in DAY mode: IR disabled initially.
    let mut ir = IrCut::new(&gpio, false)?;
    let mut mode = Mode::Day;
    ir.day();

    // --- Init I2C bus & sensors ---
    let mut i2c = I2cdev::new("/dev/i2c-1")?;

    // Sanity: read Part ID (APDS-9306-065 typically reads 0xB3)
    let part_id = als_read_u8(&mut i2c, REG_PART_ID)?;
    println!("ALS PART_ID: 0x{:02X}", part_id);

    // Optional: configure measurement rate / resolution + gain
    // als_write_u8(&mut i2c, REG_ALS_MEAS_RATE, 0x22)?; // default: 18-bit, 100ms
    // als_write_u8(&mut i2c, REG_ALS_GAIN, 0x01)?;      // default: gain 3

    // Enable ALS: set ALS_EN (bit 1) in MAIN_CTRL
    als_write_u8(&mut i2c, REG_MAIN_CTRL, 0x02)?;

    // Wait at least one integration cycle (default is ~100ms)
    thread::sleep(Duration::from_millis(150));

    // Counters for consecutive light readings
    let mut below_cnt: u8 = 0;
    let mut above_cnt: u8 = 0;
    // When we turn on IR, it adds an offset to the ambient light readings.
    let mut light_threshold_offset = 0;

    loop {
        // --- Read sensors ---
        let status = als_read_u8(&mut i2c, REG_MAIN_STATUS)?;
        let als = read_als_20bit(&mut i2c)?;
        let temp_c = read_temp_c(&mut i2c)?;

        // MAIN_STATUS bit 3 indicates "new data not yet read" (per datasheet).
        let new_data = (status & (1 << 3)) != 0;

        println!(
            "Mode={:?} MAIN_STATUS=0x{:02X} new_data={} ALS(raw20)={} Temp={:.2}°C",
            mode, status, new_data, als, temp_c
        );

        // --- Light-based hysteresis counters ---
        if als < LIGHT_THRESHOLD + light_threshold_offset {
            below_cnt = below_cnt.saturating_add(1);
            above_cnt = 0;
        } else {
            above_cnt = above_cnt.saturating_add(1);
            below_cnt = 0;
        }

        // --- Mode switching logic with temperature constraints ---

        match mode {
            Mode::Day => {
                // Switch to NIGHT only if:
                //  - dark for REQUIRED_CONSECUTIVE readings AND
                //  - temp < TEMP_THRESHOLD_C
                if below_cnt >= REQUIRED_CONSECUTIVE && temp_c < TEMP_THRESHOLD_C {
                    println!(
                        "Condition met for DAY -> NIGHT: ALS<{} for {} readings AND Temp<{:.1}°C",
                        LIGHT_THRESHOLD, REQUIRED_CONSECUTIVE, TEMP_THRESHOLD_C
                    );
                    ir.night();
                    mode = Mode::Night;
                    below_cnt = 0;
                    thread::sleep(Duration::from_millis(100));
                    light_threshold_offset = read_als_20bit(&mut i2c)?;
                    println!("light_threshold_offset set to {}", light_threshold_offset);
                }
            }
            Mode::Night => {
                // Switch to DAY if:
                //  - bright for REQUIRED_CONSECUTIVE readings OR
                //  - temp > TEMP_THRESHOLD_C
                let bright_enough = above_cnt >= REQUIRED_CONSECUTIVE;
                let too_hot = temp_c > TEMP_THRESHOLD_C;

                if bright_enough || too_hot {
                    println!(
                        "Condition met for NIGHT -> DAY: (ALS>={} for {} readings = {}) OR (Temp>{:.1}°C = {})",
                        LIGHT_THRESHOLD,
                        REQUIRED_CONSECUTIVE,
                        bright_enough,
                        TEMP_THRESHOLD_C,
                        too_hot
                    );
                    ir.day();
                    mode = Mode::Day;
                    above_cnt = 0;
                    light_threshold_offset = 0;
                }
            }
        }

        // One full iteration per second
        thread::sleep(Duration::from_secs(1));
    }
}

//fn main() -> anyhow::Result<()> {
//    let args: Vec<String> = env::args().collect();
//    if args.len() != 2 {
//        eprintln!("Usage: {} <0|1>   (0=night mode, 1=day mode)", args[0]);
//        process::exit(1);
//    }
//
//    let ir_on = match args[1].trim() {
//        "0" => true,  // night mode
//        "1" => false, // day mode
//        other => {
//            eprintln!("Invalid '{}'. Use 0 or 1.", other);
//            process::exit(1);
//        }
//    };
//
//    let gpio = Gpio::new()?;
//    let mut ir = IrCut::new(&gpio, ir_on)?;
//
//    match args[1].as_str() {
//        "0" => ir.night(),
//        "1" => ir.day(),
//        _ => unreachable!(),
//    }
//
//    Ok(())
//}
