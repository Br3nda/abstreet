extern crate abstutil;
extern crate convert_osm;
extern crate dimensioned;
extern crate gag;
extern crate geom;
extern crate sim;
extern crate yansi;

mod map_conversion;
mod physics;
mod runner;

fn main() {
    let mut t = runner::TestRunner::new();

    map_conversion::run(&mut t);
    physics::run(&mut t);

    t.done();
}