## Zabamqtt

Zabaqmtt is a full client implementation of 
the mqtt 3.1 protocol which uses currently
only the TCP stream protocol but in the 
future will also include messaging
via websocket.

The repository is very minimalistic and
has no external dependencies with
exclusion to TCPStream which is 
what I am using to implement the 
sending and receiving of msgs.


### To build
```md
cargo build
```
