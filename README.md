# Akai
![build](https://github.com/angelocarly/lov/actions/workflows/rust.yml/badge.svg)  
Generative art rendering library using [ash](https://github.com/ash-rs/ash).

## Philosophy
As this is my first delve into combining Vulkan with Rust, for that reason I'd like to keep things simple. Some points I try to consider:
- Keep the library light, don't delve into heavy abstractions early on.
- Build a stable basis with ash. I'll get things wrong and inefficient ofc, but let's make those mistakes and improve on them.
- Keep it fun and focus on art. Engine dev is cool af. But discipline and *relaxation* help eachother.

## Building & running

Make sure you have the [Vulkan SDK](https://vulkan.lunarg.com) installed.  
Then build `akai`:
```
git clone https://github.com/angelocarly/akai.git
cd akai
cargo run
```

## Libraries
- [imgui-rs-vulkan-renderer](https://github.com/adrien-ben/imgui-rs-vulkan-renderer) - TODO: Investigate adding this library
