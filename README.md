<h1 align="center">Path tracer</h1>

## Summary

A real-time 3D path tracing engine, running on the GPU, built with
[Rust](https://www.rust-lang.org/) and [wgpu](https://wgpu.rs/).

### Features

- flying camera to move around
- progressive rendering (If you stand still the engine will start to accumulate
  previous frames over time and average the pixels together, thus greatly
  reducing noise.)
- converting equirectangular HDRIs to cubemaps (which can then be used for the
  skybox)
- storing polygons in a [Bounding Volume Hierarchy](https://en.wikipedia.org/wiki/Bounding_volume_hierarchy), so they can be traversed in logarithmic time
- user interface for making changes at runtime, such as:
  - adjusting the renderer's settings
  - modifying the scene (adding and removing objects, modifying their
    properties, etc.)
- selecting objects with the cursor (this currently only works for spheres, not complex meshes)
- loading models from `.obj` files

### Future plans

- saving and loading scene data
- implementing textures
- implementing a denoiser(?)
- moving the whole thing to [Vulkan](https://www.vulkan.org/), making it possible to utilize the raytracing cores on RTX GPUs
- [DLSS](https://www.nvidia.com/en-eu/geforce/technologies/dlss/) (??)

### Running and building

You will need the Rust toolchain. (You can install it using
[rustup](https://rustup.rs).)

Running (in debug mode):

```
cargo run
```

Building in release mode:

```
cargo build --release
```

## Gallery

![image](https://github.com/landris006/path-tracer/assets/92788715/804ac2ed-2b83-48ac-b1d0-95a4d186bac2)
![image](https://github.com/landris006/path-tracer/assets/92788715/7e3d4df4-8721-4317-ac11-88ad56bb89b0)
![image](https://github.com/landris006/path-tracer/assets/92788715/22e250aa-ca1c-433d-adf6-8841d4fdcd0a)
![image](https://github.com/landris006/path-tracer/assets/92788715/c6a18880-c1f0-4db7-a8ae-01f7b149422f)
