# ongybar
Me trying to make a desktop bar for fun and to try rust

The main goal of this is to be a better version of dzen to display [monky](https://github.com/monky-hs/monky).

# This is VERY WIP.
Expect everything to not work on your system. The current code autostarts monky and uses fixed window from (0, 0) to (1366, 20).

## Goals
* Display text read over a pipe (draft)
* support multiple input streams (currently 2)
* Dynamic positioning/sizing by X events
* Custom (binary?) format for input (supported by monky)

### Secondary goals
* dynamic events for config updates during lifetime
* dynamic start/stop of additional sources

## Nongoals
* In process information gathering (outside of X)
* Interaction

## hopefully at some point
* work on wayland
* actually be fast
* Low events/time used in powertop
