# mork

### Application for sending files that is written in rust

<br>

## Usage

### Sending

`mork file-to-send`

You will get a code from the server that you can give to the reciever.

### Recieving

`mork -c <code>` *You can also set a specific output filename with the `-o` flag*

## Installation

### Cargo (Compiling it yourself)

#### Stable

You can install the binary from crates.io with cargo

`cargo install mork`

#### Experimental

You can also install from the github repository by running this.

`cargo install --git https://github.com/dojje/mork`

### Downloading directly

#### Windows

You can download the executable directly compiles for windows [here](https://github.com/dojje/mork/releases/tag/v0.1.2).

Click on `mork.exe` to download it.

But you'll realise that it's not accessable anywhere. Thats where the path global variable comes in.
To add it to the path *which means to make it accessable anywhere* you need to put the executable in any folder of you choice.

Then follow [this guide](https://medium.com/@kevinmarkvi/how-to-add-executables-to-your-path-in-windows-5ffa4ce61a53) on
how to add an executable to the system path.

## How it works

When you send a file the program will contact the tracker server which keeps track of all the codes.
The server will give you a code and store your ip.

When somebody wants to recieve your file it contacts the server with a code.
The server hands back your sender ip to the reciever and the file is sent p2p.
