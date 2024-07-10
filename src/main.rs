fn main() {
    env_logger::init();

    pollster::block_on(voxel_engine::run());

    // TODO:
    // - draw a cube
    // - make it rotate
    // - add a camera
    // - draw multiple cubes
    // - add perlin noise
    // - profit??
}
