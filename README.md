**What is this**

This program projects a photo to a ball shape. 

**Usage**

Usage: ball_image [OPTIONS] --input <INPUT> --output <OUTPUT>

Options:

    -i, --input <INPUT>        Input image path
    
    -o, --output <OUTPUT>      Output image path
    
    -s, --strength <STRENGTH>  Distortion strength (0=none, 1=normal, >1 stronger) [default: 1.0]
    
        --transparent          Transparent outside circle (default is black)
     
        --keepbox              Alternative projektion to keep horizontal and vertical are kept (default is false)
     
        --width <WIDTH>        Output width (defaults to input width)
    
        --height <HEIGHT>      Output height (defaults to input height)
  
        -h, --help             Print help

**Non-functional background**

One purpose of this project was to test latest version of AI-assisted VSCode. The first version was created with a simple prompt. Basic structure of the program was created nicely. However, it did not compile immediately, but by sending error messages to the chat fixed those errors. The main issues were with the dependencies expressed in the toml-file.

Another problen was in the algorithm - the resuling image look interesting but rather hallusinative. Since that was hard to express in the chat, I changed to manual coding. Naturally, still used the auto-completion. In that phase I learned that I have forgottent most of my knowledge in 3D-geometry :-)

Third interesting detail was the AI created a initialize multi-threading, but that was not used.
