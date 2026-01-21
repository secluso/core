use anyhow::Result;
use embedded_hal::i2c::I2c;
use linux_embedded_hal::I2cdev;
use std::{thread, time::Duration};

const ADDR: u8 = 0x52;

// Register map (APDS-9306 / APDS-9306-065)
const REG_MAIN_CTRL: u8 = 0x00;      // ALS_EN is bit 1
const REG_ALS_MEAS_RATE: u8 = 0x04;  // default 0x22
const REG_ALS_GAIN: u8 = 0x05;       // default 0x01 (gain 3)
const REG_PART_ID: u8 = 0x06;        // APDS-9306-065 default 0xB3
const REG_MAIN_STATUS: u8 = 0x07;    // ALS data status bit indicates new data
const REG_ALS_DATA_0: u8 = 0x0D;     // 0x0D..0x0F = 20-bit ALS result (LSB aligned)

fn write_u8<I: I2c>(i2c: &mut I, reg: u8, val: u8) -> core::result::Result<(), I::Error> {
    i2c.write(ADDR, &[reg, val])
}

fn read_u8<I: I2c>(i2c: &mut I, reg: u8) -> core::result::Result<u8, I::Error> {
    let mut b = [0u8; 1];
    i2c.write_read(ADDR, &[reg], &mut b)?;
    Ok(b[0])
}

fn read_als_20bit<I: I2c>(i2c: &mut I) -> core::result::Result<u32, I::Error> {
    // Block read 3 bytes starting at 0x0D to keep bytes from the same conversion.
    let mut b = [0u8; 3];
    i2c.write_read(ADDR, &[REG_ALS_DATA_0], &mut b)?;

    let raw = (b[0] as u32) | ((b[1] as u32) << 8) | (((b[2] as u32) & 0x0F) << 16);
    Ok(raw)
}

fn main() -> Result<()> {
    let mut i2c = I2cdev::new("/dev/i2c-1")?;

    // Sanity: read Part ID (APDS-9306-065 typically reads 0xB3)
    let part_id = read_u8(&mut i2c, REG_PART_ID)?;
    println!("PART_ID: 0x{:02X}", part_id);

    // Optional: configure measurement rate / resolution + gain
    // Defaults are usually fine; uncomment if you want to force them.
    // write_u8(&mut i2c, REG_ALS_MEAS_RATE, 0x22)?; // default: 18-bit, 100ms
    // write_u8(&mut i2c, REG_ALS_GAIN, 0x01)?;      // default: gain 3

    // Enable ALS: set ALS_EN (bit 1) in MAIN_CTRL
    write_u8(&mut i2c, REG_MAIN_CTRL, 0x02)?;

    // Wait at least one integration cycle (default is ~100ms)
    thread::sleep(Duration::from_millis(150));

    loop {
        let status = read_u8(&mut i2c, REG_MAIN_STATUS)?;
        let als = read_als_20bit(&mut i2c)?;

        // MAIN_STATUS bit 3 indicates "new data not yet read" (per datasheet).
        let new_data = (status & (1 << 3)) != 0;

        println!("MAIN_STATUS=0x{:02X} new_data={} ALS(raw20)={}", status, new_data, als);

        thread::sleep(Duration::from_millis(500));
    }
}

