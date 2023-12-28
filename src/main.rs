use pathtracer::run;

fn main() {
    pollster::block_on(run());
}
