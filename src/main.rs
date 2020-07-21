use std::thread;
use std::time::Duration;

use feeder::Feeder;
use gcfeeder::feeder;

fn main() -> Result<(), feeder::Error> {
    let mut feeder = Feeder::new()?;

    loop {
        feeder.feed()?;
        thread::sleep(Duration::from_millis(2));
    }
}
