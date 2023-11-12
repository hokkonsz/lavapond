![lavapond](https://github.com/hokkonsz/lavapond/assets/54407548/221d3589-282c-48cc-afa2-8181a0e7b332)

[![dependency status](https://deps.rs/repo/github/hokkonsz/lavapond/status.svg)](https://deps.rs/repo/github/hokkonsz/lavapond)

Learning project for graphics programming using Vulkan with Ash in Rust.
Here I collected all the useful resources I used through my journey up until this point.
It may also be useful for others who have stumbled across it.

### VULKAN

I started with the most recommended [Vulkan tutorial](https://vulkan-tutorial.com/Drawing_a_triangle/Drawing/Rendering_and_presentation) by Alexander Overvoorde.
There is also a [licensed version](https://docs.vulkan.org/tutorial/latest/00_Introduction.html) of this on the officialy Khronos site,
but because I used the one created by Alexander I can't really compare the two.
Although the unofficaly one has a [Rust implementation](https://github.com/bwasty/vulkan-tutorial-rs) I didn't really used it, because it uses Vulkano instead of Ash.
Speaking of Ash, the [exmaple project](https://github.com/ash-rs/ash/tree/master/examples) in Ash was also very useful
and I would recommend it, if you have difficulties implementing the C++ code in Rust.

Understanding every aspect of the tutorial could also be problematic. Fortunelty there are a plenty of resources on Vulkan.
For example there is a free video series about [Vulkan Essentials](https://www.youtube.com/watch?v=tLwbj9qys18&list=PLmIqTlJ6KsE1Jx5HV4sd2jOe3V1KMHHgn) on the Computer Graphics at TU Wien channel, created by Johannes Unterguggenberger.
If at any steps you have difficulties with understanding a part of Vulkan or something is not clear, then I recommend to check the official [Vulkan Specification](https://registry.khronos.org/vulkan/specs/1.3-extensions/html/vkspec.html).
You can also find many [offical examples](https://github.com/KhronosGroup/Vulkan-Samples) under the Khronos Group github.

### MATH / COMPUTER GRAPHICS

Math and especially matrices are playing a great role in throughly understanding computer graphics.
For example I used the [matrices tutorial](https://www.opengl-tutorial.org/beginners-tutorials/tutorial-3-matrices/) on opengl-tutorial.org, to refresh my knowledge on the subject.
But I found the [Learning Modern 3D Graphics Programming](https://paroj.github.io/gltut/) by Jason L. McKesson also really helpful for
learning about general topics in computer graphics, which cannot be learned by only reading the Vulkan tutorial or Vulkan specifications.

### NEXT STEPS

After all of this you will have a base knowledge of how Vulkan and computer graphics works. If you want to step up your game, then there
are other useful resources, like [Writing an efficient Vulkan renderer](https://zeux.io/2020/02/27/writing-an-efficient-vulkan-renderer/) by Arseny Kapoulkine.
For anyone who is interested in vector graphics there is [Vector graphics on GPU](https://gasiulis.name/vector-graphics-on-gpu/) by Aurimas Gasiulis.
I also found interesenting of the [Drawing Antialiased Lines with OpenGL](https://blog.mapbox.com/drawing-antialiased-lines-with-opengl-8766f34192dc) by Konstantin Käfer.
Maybe reading these will put an idea in your head about where to go next.
