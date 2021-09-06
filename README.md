# An unnamed project

A wayland compositor and applications. At the moment I have no name for this, but I have a few ideas and goals:

Compositor:

- The compositor will be rendered using Vulkan, probably will use EGL till this is working in Smithay.
- Experiment with the idea for a globally accessible panel containing some toplevel surfaces, something I could store a transient browser with matrix open or discord and hide/open on demand.

Applications:

- Some applications such as a music player may need to be homegrown to be accessible via layer popups, this will go in the `applications` folder at some point.
- Vulkan based background layer renderer using layer shell. This application would be configured via some textures and shaders.

Other:

- Find some way to generate GTK and Qt themes so those applications look consistent with the compositor.
