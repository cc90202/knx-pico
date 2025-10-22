#!/usr/bin/env python3
"""
knx_search.py
Semplice client KNXnet/IP Search (SEARCH) scritto in Python.
- invia una SearchRequest (service 0x0201) con HPAI IPv4/UDP
- invia a multicast 224.0.23.12:3671 e a broadcast
- ascolta SearchResponse (0x0202) per un timeout

Uso:
    python3 knx_search.py [--timeout SECS] [--iface IFACE_IP]

"""
import argparse
import socket
import struct
import sys
import time

KNX_MCAST = ('224.0.23.12', 3671)
KNX_PORT = 3671


def build_search_request(hpai_ip, hpai_port):
    # KNXnet/IP header: header_len(1), protocol_version(1), service_type(2), total_length(2)
    # payload: HPAI (len=8, protocol=1, ipv4(4), port(2))
    header_len = 0x06
    protocol_version = 0x10
    service_type = 0x0201
    payload = bytearray()
    payload.append(8)  # HPAI length
    payload.append(1)  # IPv4 UDP
    payload.extend(socket.inet_aton(hpai_ip))
    payload.extend(struct.pack('!H', hpai_port))
    total_length = 6 + len(payload)
    buf = bytearray()
    buf.append(header_len)  # IMPORTANTE: primo byte deve essere header_len
    buf.append(protocol_version)
    buf.extend(struct.pack('!H', service_type))
    buf.extend(struct.pack('!H', total_length))
    buf.extend(payload)
    return bytes(buf)


def parse_search_response(buf):
    # Parse KNXnet/IP header: header_len(1), protocol_version(1), service_type(2), total_length(2)
    if len(buf) < 6:
        return None
    header_len = buf[0]
    if header_len != 0x06:
        return None
    if buf[1] != 0x10:  # protocol version
        return None
    service_type = struct.unpack('!H', buf[2:4])[0]
    if service_type != 0x0202:
        return None
    total_length = struct.unpack('!H', buf[4:6])[0]
    if total_length > len(buf):
        return None
    # Search for HPAI TLV (len=8, proto=1) starting from body (offset 6)
    for i in range(6, len(buf)-7):
        if buf[i] == 8 and buf[i+1] == 1:
            ip = socket.inet_ntoa(buf[i+2:i+6])
            port = struct.unpack('!H', buf[i+6:i+8])[0]
            return (ip, port)
    return None


def get_local_ip_by_connect(remote=('8.8.8.8', 80)):
    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    try:
        s.connect(remote)
        local = s.getsockname()[0]
        return local
    except Exception:
        return '0.0.0.0'
    finally:
        s.close()


def make_recv_socket(bind_port, iface_ip=None):
    # Create UDP socket and try to bind to bind_port with reuse options.
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    if hasattr(socket, 'SO_REUSEPORT'):
        try:
            sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEPORT, 1)
        except Exception:
            pass
    try:
        sock.bind(('0.0.0.0', bind_port))
    except OSError as e:
        # fallback to ephemeral
        sock.close()
        sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        if hasattr(socket, 'SO_REUSEPORT'):
            try:
                sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEPORT, 1)
            except Exception:
                pass
        sock.bind(('0.0.0.0', 0))
        return sock

    # join multicast group if iface provided
    if iface_ip and iface_ip != '0.0.0.0':
        mreq = struct.pack('4s4s', socket.inet_aton(KNX_MCAST[0]), socket.inet_aton(iface_ip))
        try:
            sock.setsockopt(socket.IPPROTO_IP, socket.IP_ADD_MEMBERSHIP, mreq)
        except Exception:
            # best effort
            pass
    return sock


def hex_of(b):
    return ''.join(f"{x:02x}" for x in b)


def build_search_response(server_ip, server_port):
    # Build a minimal SearchResponse: protocol 0x10, service 0x0202, total length = 5 + payload
    protocol_version = 0x10
    service_type = 0x0202
    payload = bytearray()
    # include HPAI for the server
    payload.append(8)
    payload.append(1)
    payload.extend(socket.inet_aton(server_ip))
    payload.extend(struct.pack('!H', server_port))
    total_length = 6 + len(payload)  # match simulator's expectation
    buf = bytearray()
    buf.append(protocol_version)
    buf.extend(struct.pack('!H', service_type))
    buf.extend(struct.pack('!H', total_length))
    buf.extend(payload)
    return bytes(buf)


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--timeout', '-t', type=float, default=3.0, help='timeout in seconds')
    p.add_argument('--iface', '-i', help='local interface IPv4 address to use (optional)')
    p.add_argument('--repeat', '-r', type=int, default=1, help='how many times to send the SearchRequest')
    p.add_argument('--simulate-response', action='store_true', help='send a fake SearchResponse to the recv socket for testing')
    args = p.parse_args()

    local_ip = args.iface or get_local_ip_by_connect()
    print(f'Local IP for HPAI: {local_ip}')

    recv_sock = make_recv_socket(KNX_PORT, iface_ip=local_ip)
    recv_port = recv_sock.getsockname()[1]
    print(f'Receive socket bound to port: {recv_port}')

    # allow broadcast on the same socket
    try:
        recv_sock.setsockopt(socket.SOL_SOCKET, socket.SO_BROADCAST, 1)
    except Exception:
        pass

    req = build_search_request(local_ip, recv_port)
    print('SearchRequest hex:', hex_of(req))

    # sender socket
    send_sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    # set multicast outgoing iface
    if local_ip and local_ip != '0.0.0.0':
        try:
            send_sock.setsockopt(socket.IPPROTO_IP, socket.IP_MULTICAST_IF, socket.inet_aton(local_ip))
        except Exception:
            pass
    send_sock.setsockopt(socket.SOL_SOCKET, socket.SO_BROADCAST, 1)

    # send to multicast
    try:
        send_sock.sendto(req, KNX_MCAST)
        print(f'Sent {len(req)} bytes to {KNX_MCAST[0]}:{KNX_MCAST[1]}')
    except Exception as e:
        print('Error sending multicast:', e)

    # send to global broadcast
    try:
        send_sock.sendto(req, ('255.255.255.255', KNX_PORT))
        print(f'Sent {len(req)} bytes to 255.255.255.255:{KNX_PORT}')
    except Exception as e:
        print('Error sending broadcast:', e)

    # send to local subnet broadcast x.y.z.255
    if local_ip and local_ip != '0.0.0.0':
        octs = local_ip.split('.')
        if len(octs) == 4:
            local_bcast = f'{octs[0]}.{octs[1]}.{octs[2]}.255'
            try:
                send_sock.sendto(req, (local_bcast, KNX_PORT))
                print(f'Sent {len(req)} bytes to {local_bcast}:{KNX_PORT}')
            except Exception as e:
                print('Error sending local broadcast:', e)

    # If simulate-response is enabled, send a fake SearchResponse back to our recv socket
    if args.simulate_response:
        # build response where server is local_ip:KNX_PORT
        resp = build_search_response(local_ip, KNX_PORT)
        time.sleep(0.2)
        try:
            send_sock.sendto(resp, ('127.0.0.1', recv_port))
            # also try sending to local_ip in case recv binds to external address
            send_sock.sendto(resp, (local_ip, recv_port))
            print(f'Sent simulated SearchResponse to {local_ip}:{recv_port} and 127.0.0.1:{recv_port}')
        except Exception as e:
            print('Error sending simulated response:', e)

    send_sock.close()

    # receive responses
    recv_sock.settimeout(args.timeout)
    start = time.time()
    found = []
    while True:
        try:
            data, addr = recv_sock.recvfrom(2048)
        except socket.timeout:
            break
        except Exception as e:
            print('Recv error:', e)
            break
        print(f'Received {len(data)} bytes from {addr}: hex={hex_of(data)}')
        pr = parse_search_response(data)
        if pr:
            if pr not in found:
                found.append(pr)
                print('Found KNX device:', pr[0], pr[1])
        # stop if time exceeded
        if time.time() - start >= args.timeout:
            break

    if not found:
        print(f'No KNX devices found within {args.timeout}s')
    else:
        print(f'Found {len(found)} device(s)')


if __name__ == '__main__':
    main()
