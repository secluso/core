use anyhow::Result;
use embedded_hal::i2c::I2c;
use linux_embedded_hal::I2cdev;
use std::{thread, time::Duration};

const ADDR: u8 = 0x48;      // TMP1075 default address
const REG_TEMP: u8 = 0x00;   // Temperature register

fn read_temp_raw<I: I2c>(i2c: &mut I) -> core::result::Result<i16, I::Error> {
    let mut b = [0u8; 2];
    i2c.write_read(ADDR, &[REG_TEMP], &mut b)?;

    let raw = i16::from_be_bytes(b);

    // 12-bit value in upper bits
    Ok(raw >> 4)
}

fn main() -> Result<()> {
    let mut i2c = I2cdev::new("/dev/i2c-1")?;

    loop {
        let raw12 = read_temp_raw(&mut i2c)?;
        let temp_c = raw12 as f32 / 16.0;

        println!("Temperature: {:.4} °C (raw={})", temp_c, raw12);

        thread::sleep(Duration::from_secs(1));
    }
}
