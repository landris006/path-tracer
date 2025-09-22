<h1 align="center">Path tracer</h1>

## Summary

A real-time 3D path tracing engine. Built with
[Rust](https://www.rust-lang.org/) and [wgpu](https://wgpu.rs/).

### Features

- flying camera to move around
- progressive rendering (If you stand still the engine will start to accumulate
  previous frames over time and average the pixels together. This is a form of noise reduction.)
- convert equirectangular HDRIs to cubemaps (useful for rendering the sky)
- [Bounding Volume Hierarchy](https://en.wikipedia.org/wiki/Bounding_volume_hierarchy) for traversing meshes in logarithmic time
- user interface for making changes at runtime, such as:
  - adjusting the renderer's settings
  - modifying the scene (adding and removing objects, modifying their
    properties, etc.)
- select objects with the cursor (this currently only works for spheres, not complex meshes)
- load models from `.obj` files

### Future plans

- saving and loading scene data
- implementing textures
- implementing a denoiser(?)
- moving the whole thing to [Vulkan](https://www.vulkan.org/) and utilize the raytracing cores on RTX GPUs
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

## Progressive rendering demo
https://github.com/user-attachments/assets/395f185c-1256-4ab7-8da1-8106761f9059

## Gallery
![bunny](https://github.com/landris006/path-tracer/assets/92788715/fffb3be0-3318-46c0-a111-ab9db1061308)
![cornell-2](https://github.com/landris006/path-tracer/assets/92788715/28329748-8de9-4b88-b15d-da986cd2ccd0)
![cornell](https://github.com/landris006/path-tracer/assets/92788715/c202ea74-76f5-40c3-9be7-09a71c1e387b)
![3-balls](https://github.com/landris006/path-tracer/assets/92788715/2aee6573-77cd-4efa-8e80-d960032b1db6)
![image](https://github.com/landris006/path-tracer/assets/92788715/804ac2ed-2b83-48ac-b1d0-95a4d186bac2)
![image](https://github.com/landris006/path-tracer/assets/92788715/7e3d4df4-8721-4317-ac11-88ad56bb89b0)
![image](https://github.com/landris006/path-tracer/assets/92788715/22e250aa-ca1c-433d-adf6-8841d4fdcd0a)
![image](https://github.com/landris006/path-tracer/assets/92788715/c6a18880-c1f0-4db7-a8ae-01f7b149422f)
