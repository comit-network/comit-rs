#!/usr/bin/env bash
# This captures all the traffic on the docker network interface using tcpdump and writes it to a trace file in the same
# folder as the logs of all the docker containers.

nohup tcpdump -i docker0 tcp -U -w /tmp/build_$TRAVIS_COMMIT/docker_trace.pcap &
