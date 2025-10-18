#!/usr/bin/env python3
"""
Simple KNXnet/IP Gateway Simulator for testing knx-rs

This simulates a basic KNXnet/IP gateway that responds to:
- CONNECT_REQUEST
- DISCONNECT_REQUEST
- TUNNELING_REQUEST
- CONNECTIONSTATE_REQUEST

Usage:
    python3 knx_simulator.py [--port 3671] [--verbose]
"""

import socket
import struct
import argparse
from datetime import datetime

# KNXnet/IP Service Type Identifiers
SERVICE_CONNECT_REQUEST = 0x0205
SERVICE_CONNECT_RESPONSE = 0x0206
SERVICE_CONNECTIONSTATE_REQUEST = 0x0207
SERVICE_CONNECTIONSTATE_RESPONSE = 0x0208
SERVICE_DISCONNECT_REQUEST = 0x0209
SERVICE_DISCONNECT_RESPONSE = 0x020A
SERVICE_TUNNELING_REQUEST = 0x0420
SERVICE_TUNNELING_ACK = 0x0421
SERVICE_TUNNELING_INDICATION = 0x0420  # Same as REQUEST but direction is gateway->client

# Status codes
STATUS_OK = 0x00
STATUS_NO_ERROR = 0x00

class KNXSimulator:
    def __init__(self, port=3671, verbose=False):
        self.port = port
        self.verbose = verbose
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        self.sock.bind(('0.0.0.0', port))
        self.channels = {}  # channel_id -> (client_addr, sequence_counter)
        self.next_channel = 1

    def log(self, msg):
        if self.verbose:
            timestamp = datetime.now().strftime("%H:%M:%S.%f")[:-3]
            print(f"[{timestamp}] {msg}")

    def parse_header(self, data):
        """Parse KNXnet/IP header"""
        if len(data) < 6:
            return None

        header_len, protocol_version, service_type, total_len = struct.unpack('>BBHH', data[:6])

        return {
            'header_len': header_len,
            'version': protocol_version,
            'service_type': service_type,
            'total_len': total_len,
            'body': data[6:]
        }

    def build_header(self, service_type, body_len):
        """Build KNXnet/IP header"""
        total_len = 6 + body_len
        return struct.pack('>BBHH', 0x06, 0x10, service_type, total_len)

    def handle_connect_request(self, data, client_addr):
        """Handle CONNECT_REQUEST"""
        self.log(f"CONNECT_REQUEST from {client_addr}")

        # Assign channel with sequence counter starting at 0
        channel_id = self.next_channel
        self.channels[channel_id] = (client_addr, 0)
        self.next_channel += 1

        # Build CONNECT_RESPONSE
        # Channel ID, Status
        body = struct.pack('BB', channel_id, STATUS_OK)
        # HPAI (8 bytes) - Control endpoint
        body += struct.pack('>BB4sH', 0x08, 0x01, b'\x00\x00\x00\x00', 0)
        # CRD (Connection Response Data) - 4 bytes
        body += struct.pack('>BBH', 0x04, 0x04, 0x0200)

        header = self.build_header(SERVICE_CONNECT_RESPONSE, len(body))
        response = header + body

        self.log(f"  → CONNECT_RESPONSE: channel={channel_id}")
        return response

    def handle_disconnect_request(self, data, client_addr):
        """Handle DISCONNECT_REQUEST"""
        if len(data) < 2:
            return None

        channel_id, status = struct.unpack('BB', data[:2])
        self.log(f"DISCONNECT_REQUEST: channel={channel_id}")

        # Remove channel
        if channel_id in self.channels:
            del self.channels[channel_id]

        # Build DISCONNECT_RESPONSE
        body = struct.pack('BB', channel_id, STATUS_OK)
        header = self.build_header(SERVICE_DISCONNECT_RESPONSE, len(body))
        response = header + body

        self.log(f"  → DISCONNECT_RESPONSE")
        return response

    def handle_tunneling_request(self, data, client_addr):
        """Handle TUNNELING_REQUEST"""
        if len(data) < 4:
            return None

        # Connection header: len, channel, sequence, reserved
        conn_header = data[:4]
        header_len, channel_id, sequence, reserved = struct.unpack('BBBB', conn_header)
        cemi_data = data[4:]

        self.log(f"TUNNELING_REQUEST: channel={channel_id}, seq={sequence}, cemi_len={len(cemi_data)}")

        # Build TUNNELING_ACK
        body = conn_header + struct.pack('B', STATUS_OK)  # Connection header + status
        header = self.build_header(SERVICE_TUNNELING_ACK, len(body))
        response = header + body

        self.log(f"  → TUNNELING_ACK: seq={sequence}")

        # After receiving 2nd request, simulate bus event
        # Send a TUNNELING_INDICATION back to test bidirectional communication
        if sequence == 1:  # After client sends 2nd command (OFF)
            import time
            time.sleep(0.5)  # Small delay
            # Simulate: Light at 1/2/4 was turned ON by another device
            cemi = self.build_cemi_group_write(0x0A04, True)  # 1/2/4 = ON
            self.send_tunneling_indication(channel_id, cemi)
            self.log(f"  💡 Simulated bus event: Light 1/2/4 turned ON")

        return response

    def handle_connectionstate_request(self, data, client_addr):
        """Handle CONNECTIONSTATE_REQUEST (heartbeat)"""
        if len(data) < 2:
            return None

        channel_id, reserved = struct.unpack('BB', data[:2])
        self.log(f"CONNECTIONSTATE_REQUEST: channel={channel_id}")

        # Build CONNECTIONSTATE_RESPONSE
        body = struct.pack('BB', channel_id, STATUS_OK)
        header = self.build_header(SERVICE_CONNECTIONSTATE_RESPONSE, len(body))
        response = header + body

        self.log(f"  → CONNECTIONSTATE_RESPONSE")
        return response

    def build_cemi_group_write(self, group_addr, value_bool):
        """Build a cEMI L_Data.ind frame for GroupValue_Write"""
        cemi = bytearray()

        # Message code: L_Data.ind (0x29)
        cemi.append(0x29)

        # Additional info length: 0
        cemi.append(0x00)

        # Control field 1: Standard frame
        cemi.append(0xBC)

        # Control field 2: Group address, hop count 6
        cemi.append(0xE0)

        # Source address: 1.1.250 (gateway)
        cemi.extend([0x11, 0xFA])

        # Destination address: group address (2 bytes, big endian)
        cemi.extend(struct.pack('>H', group_addr))

        # NPDU length: 1
        cemi.append(0x01)

        # TPCI/APCI
        cemi.append(0x00)

        # APCI + 6-bit data (GroupValueWrite = 0x80)
        apci_data = 0x81 if value_bool else 0x80
        cemi.append(apci_data)

        return bytes(cemi)

    def send_tunneling_indication(self, channel_id, cemi_data):
        """Send TUNNELING_INDICATION to client"""
        if channel_id not in self.channels:
            return

        client_addr, sequence = self.channels[channel_id]

        # Build connection header (4 bytes)
        conn_header = struct.pack('BBBB', 0x04, channel_id, sequence, 0x00)

        # Build body: connection header + cEMI
        body = conn_header + cemi_data

        # Build frame
        header = self.build_header(SERVICE_TUNNELING_INDICATION, len(body))
        frame = header + body

        # Send
        self.sock.sendto(frame, client_addr)

        # Update sequence counter
        self.channels[channel_id] = (client_addr, (sequence + 1) % 256)

        self.log(f"  → TUNNELING_INDICATION: channel={channel_id}, seq={sequence}, cemi_len={len(cemi_data)}")

    def run(self):
        """Main server loop"""
        print(f"=== KNX Gateway Simulator ===")
        print(f"Listening on 0.0.0.0:{self.port}")
        print(f"Gateway address: 1.1.250")
        print(f"Client addresses: 1.1.128 - 1.1.135")
        print(f"Press Ctrl+C to stop\n")

        try:
            while True:
                data, client_addr = self.sock.recvfrom(1024)

                # RAW debug: always print received data
                print(f"\n[RAW] Received {len(data)} bytes from {client_addr}")
                print(f"      Hex: {data.hex()}")

                # Parse header
                frame = self.parse_header(data)
                if not frame:
                    self.log(f"Invalid frame from {client_addr}")
                    print(f"      ERROR: Failed to parse header")
                    continue

                # Handle request
                response = None
                service_type = frame['service_type']

                if service_type == SERVICE_CONNECT_REQUEST:
                    response = self.handle_connect_request(frame['body'], client_addr)
                elif service_type == SERVICE_DISCONNECT_REQUEST:
                    response = self.handle_disconnect_request(frame['body'], client_addr)
                elif service_type == SERVICE_TUNNELING_REQUEST:
                    response = self.handle_tunneling_request(frame['body'], client_addr)
                elif service_type == SERVICE_CONNECTIONSTATE_REQUEST:
                    response = self.handle_connectionstate_request(frame['body'], client_addr)
                else:
                    self.log(f"Unknown service type: 0x{service_type:04X}")

                # Send response
                if response:
                    self.sock.sendto(response, client_addr)

        except KeyboardInterrupt:
            print("\n\nShutting down...")
        finally:
            self.sock.close()

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description='KNXnet/IP Gateway Simulator')
    parser.add_argument('--port', type=int, default=3671, help='UDP port (default: 3671)')
    parser.add_argument('-v', '--verbose', action='store_true', help='Verbose logging')

    args = parser.parse_args()

    simulator = KNXSimulator(port=args.port, verbose=args.verbose)
    simulator.run()
