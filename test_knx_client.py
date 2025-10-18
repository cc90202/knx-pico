#!/usr/bin/env python3
"""
Simple KNX client to test the simulator
Sends CONNECT_REQUEST to the gateway
"""

import socket
import struct

# KNX Gateway
GATEWAY_IP = "192.168.1.23"
GATEWAY_PORT = 3671

# Service types
SERVICE_CONNECT_REQUEST = 0x0205

def build_connect_request():
    """Build a CONNECT_REQUEST frame"""
    # Header: len(6), version(0x10), service_type(0x0205), total_len
    header = struct.pack('>BBHH', 0x06, 0x10, SERVICE_CONNECT_REQUEST, 26)

    # Control endpoint HPAI (8 bytes): len, proto, IP, port
    # Use 0.0.0.0:0 for NAT mode
    control_hpai = struct.pack('>BB4sH', 0x08, 0x01, b'\x00\x00\x00\x00', 0)

    # Data endpoint HPAI (8 bytes)
    data_hpai = struct.pack('>BB4sH', 0x08, 0x01, b'\x00\x00\x00\x00', 0)

    # CRI (Connection Request Information) - 4 bytes
    # len, connection_type(TUNNEL_CONNECTION=0x04), knx_layer(TUNNEL_LINKLAYER=0x02), reserved
    cri = struct.pack('>BBBB', 0x04, 0x04, 0x02, 0x00)

    return header + control_hpai + data_hpai + cri

def parse_connect_response(data):
    """Parse CONNECT_RESPONSE"""
    if len(data) < 8:
        return None

    header_len, version, service_type, total_len = struct.unpack('>BBHH', data[:6])

    print(f"  Header: len={header_len}, version=0x{version:02x}, service=0x{service_type:04x}, total={total_len}")

    if len(data) >= 8:
        channel_id, status = struct.unpack('BB', data[6:8])
        print(f"  Channel ID: {channel_id}")
        print(f"  Status: {status} ({'OK' if status == 0 else 'ERROR'})")
        return channel_id if status == 0 else None

    return None

def main():
    print("=== KNX Client Test ===")
    print(f"Connecting to {GATEWAY_IP}:{GATEWAY_PORT}\n")

    # Create UDP socket
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.settimeout(5.0)

    try:
        # Build and send CONNECT_REQUEST
        connect_req = build_connect_request()
        print(f"Sending CONNECT_REQUEST ({len(connect_req)} bytes)")
        print(f"  Hex: {connect_req.hex()}")

        sock.sendto(connect_req, (GATEWAY_IP, GATEWAY_PORT))
        print("✓ Sent\n")

        # Wait for CONNECT_RESPONSE
        print("Waiting for CONNECT_RESPONSE...")
        data, addr = sock.recvfrom(1024)

        print(f"\n✓ Received {len(data)} bytes from {addr}")
        print(f"  Hex: {data.hex()}")

        # Parse response
        channel_id = parse_connect_response(data)

        if channel_id:
            print(f"\n✅ SUCCESS! Connected with channel ID {channel_id}")
        else:
            print("\n❌ FAILED: Invalid response or error status")

    except socket.timeout:
        print("\n❌ TIMEOUT: No response from gateway")
    except Exception as e:
        print(f"\n❌ ERROR: {e}")
    finally:
        sock.close()

if __name__ == '__main__':
    main()
