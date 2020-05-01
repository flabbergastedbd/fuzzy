# vim: set tabstop=8 softtabstop=0 expandtab shiftwidth=4 smarttab:
@0xe8956e5f92e8a6f3;

struct Pulse {
    cpus @0 :UInt8;
    workerId @1 :Data;
    name @2 :Text;
}

interface PulseTracker {
    addPulse @0 (p :Pulse) -> (result :Bool);
}
