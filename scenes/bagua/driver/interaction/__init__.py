from . import input, copying, take, recovery


def run(host):
    input.run(host)
    copying.run(host)
    take.run(host)
    recovery.run(host)
