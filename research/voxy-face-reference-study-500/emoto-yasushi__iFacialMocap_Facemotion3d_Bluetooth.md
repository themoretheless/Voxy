# Facial rendering source scan: emoto-yasushi/iFacialMocap_Facemotion3d_Bluetooth

Repository: https://github.com/emoto-yasushi/iFacialMocap_Facemotion3d_Bluetooth

Pinned source revision: c35400a9fd892a54c2eb859155fd0ad020bf63bb

Discovery topic: facial mocap

Description: 

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"tooth": ["bluetooth_sample_code.py"]}

## Source file: bluetooth_sample_code.py

{
  "path": "bluetooth_sample_code.py",
  "blob_sha": "a8ea1e87d6b06277b85bfed94f87b817c3429ca8",
  "sha256": "76385b0e909d2ef7fec1525f6ca6ac5d041f316eeffab35a9ae18ae64d6cc7e6",
  "lines": 1276,
  "bytes": 45082,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 8,
    "blend_shape": 7,
    "tooth": 119,
    "rig": 14,
    "face": 10
  },
  "url": "https://github.com/emoto-yasushi/iFacialMocap_Facemotion3d_Bluetooth/blob/c35400a9fd892a54c2eb859155fd0ad020bf63bb/bluetooth_sample_code.py",
  "function_signature_candidates": [
    {
      "line": 73,
      "text": "def __init__(self, bluetooth_id=\"default\", message_mode=\"iFacialMocap\"):"
    },
    {
      "line": 133,
      "text": "def decode_blendshapes_data(self, data: bytes, index: int) -> Tuple[str, int]:"
    },
    {
      "line": 153,
      "text": "def decode_bones_data_0x02(self, data: bytes, index: int) -> Tuple[str, int]:"
    },
    {
      "line": 181,
      "text": "def decode_bones_data_0x03(self, data: bytes, index: int) -> Tuple[str, int]:"
    },
    {
      "line": 204,
      "text": "def values_decode_data(self, data: bytes, data_size: int) -> str:"
    },
    {
      "line": 240,
      "text": "def decompress_zlib(self, compressed_data: bytes, data_size=None) -> Optional[str]:"
    },
    {
      "line": 248,
      "text": "def generate_uuid_v5_from_string(self, namespace_uuid, input_string):"
    },
    {
      "line": 255,
      "text": "def _reset_receive_parser(self, clear_schema=False):"
    },
    {
      "line": 262,
      "text": "def _publish_received_message(self, message: str):"
    },
    {
      "line": 284,
      "text": "def _handle_notification("
    },
    {
      "line": 351,
      "text": "def get_received_message(self, timeout=None) -> str:"
    },
    {
      "line": 368,
      "text": "def received_message(self):"
    },
    {
      "line": 371,
      "text": "def clear_received_messages(self):"
    },
    {
      "line": 387,
      "text": "def _last_addr_path(self) -> str:"
    },
    {
      "line": 398,
      "text": "def _load_last_addr(self) -> Optional[str]:"
    },
    {
      "line": 406,
      "text": "def _save_last_addr(self, address: str):"
    },
    {
      "line": 415,
      "text": "def _name_matches(cls, device_name: Optional[str]) -> bool:"
    },
    {
      "line": 427,
      "text": "def _entry_key(device) -> str:"
    },
    {
      "line": 434,
      "text": "def _entry_name(device, advertisement_data) -> str:"
    },
    {
      "line": 448,
      "text": "def _entry_advertises_target_service(self, advertisement_data) -> bool:"
    },
    {
      "line": 456,
      "text": "async def _discover_entries("
    },
    {
      "line": 499,
      "text": "def _order_candidates("
    },
    {
      "line": 508,
      "text": "def add_matching(predicate):"
    },
    {
      "line": 561,
      "text": "async def _scan_candidates(self) -> List[Any]:"
    },
    {
      "line": 606,
      "text": "async def _has_required_characteristics(self, client) -> bool:"
    },
    {
      "line": 650,
      "text": "def _client_is_connected(client) -> bool:"
    },
    {
      "line": 658,
      "text": "def _on_client_disconnected(self, client):"
    },
    {
      "line": 667,
      "text": "async def _disconnect_client(self, client=None):"
    },
    {
      "line": 688,
      "text": "async def _ensure_connected(self) -> bool:"
    },
    {
      "line": 796,
      "text": "def _is_stop_streaming_command(message: str) -> bool:"
    },
    {
      "line": 800,
      "text": "def _is_stream_start_command(message: str) -> bool:"
    },
    {
      "line": 803,
      "text": "async def _write_protocol_message("
    },
    {
      "line": 845,
      "text": "async def bluetooth_send_message(self, message):"
    },
    {
      "line": 854,
      "text": "async def bluetooth_recieve_realtime_start(self, message):"
    },
    {
      "line": 864,
      "text": "async def _send_command_with_retry(self, message: str) -> bool:"
    },
    {
      "line": 884,
      "text": "async def _command_worker(self):"
    },
    {
      "line": 896,
      "text": "async def _keepalive_worker(self):"
    },
    {
      "line": 930,
      "text": "def _loop_thread_main(self):"
    },
    {
      "line": 992,
      "text": "def _start_loop_thread(self):"
    },
    {
      "line": 1016,
      "text": "def _enqueue_async_command(self, message: str):"
    },
    {
      "line": 1026,
      "text": "def bluetooth_send_message_trigger(self, message):"
    },
    {
      "line": 1044,
      "text": "def process_message_queue(self):"
    },
    {
      "line": 1074,
      "text": "def run_bluetooth_loop(self, message):"
    },
    {
      "line": 1079,
      "text": "async def keep_bluetooth_alive(self, message):"
    },
    {
      "line": 1088,
      "text": "def wait_for_all_commands(self, timeout=5.0) -> bool:"
    },
    {
      "line": 1097,
      "text": "async def wait_for_queue_barrier():"
    },
    {
      "line": 1119,
      "text": "async def stop_async_tasks(self):"
    },
    {
      "line": 1139,
      "text": "async def stop_notifications_and_cleanup(self):"
    },
    {
      "line": 1142,
      "text": "async def _shutdown_async(self):"
    },
    {
      "line": 1146,
      "text": "def _clear_queue(self, target_queue):"
    }
  ]
}

```text

"""
Created by Yasushi Emoto
4/4/2025

This Python script enables real-time communication using iFacialMocap,
Facemotion3d, and Bluetooth Low Energy (BLE).

Install Bleak before running:
    pip install "bleak>=0.19,<4"

Facemotion3d may send negative BlendShape values. iFacialMocap does not use
negative BlendShape values.

    bluetooth_instance.message_mode = "iFacialMocap"
        Negative BlendShape values are clamped to 0 and the delimiter is "-".

    bluetooth_instance.message_mode = "Facemotion3d"
        Negative BlendShape values are preserved and the delimiter is "&".

Compatibility notes
-------------------
The original public class name and methods are intentionally preserved:

    ifm_and_fm_bluetooh_functions
    bluetooth_send_message_trigger(...)
    bluetooth_stop()
    recieved_message

For reliable, non-duplicated reception, use get_received_message() instead of
polling recieved_message repeatedly.
"""

import asyncio
import json
import os
import queue
import struct
import threading
import traceback
import uuid
import zlib
from concurrent.futures import CancelledError as FuturesCancelledError
from concurrent.futures import TimeoutError as FuturesTimeoutError
from typing import Any, List, Optional, Sequence, Tuple

from bleak import BleakClient, BleakGATTCharacteristic, BleakScanner


class ifm_and_fm_bluetooh_functions:
    """Bluetooth receiver shared by iFacialMocap and Facemotion3d.

    The misspelled class name is retained for compatibility with existing code.
    """

    SERVICE_NAMESPACE_UUID = "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
    NOTIFY_CHARACTERISTIC_UUID = "EFAB5678-1234-90AB-CDEF-1234567890AB"
    WRITE_CHARACTERISTIC_UUID = "ABCD1234-5678-90AB-CDEF-1234567890AB"

    ACCEPTED_NAMES = {
        "ifacialmocap-ios",
        "facemotion-ios",
        "facemotion3d-ios",
    }
    ACCEPTED_PREFIXES = ("ifacialmocap", "facemotion")

    TARGETED_SCAN_SECONDS = 4.0
    FALLBACK_SCAN_SECONDS = 5.0
    FIRST_CONNECT_TIMEOUT_SECONDS = 10.0
    OTHER_CONNECT_TIMEOUT_SECONDS = 7.0
    MAX_UNKNOWN_CANDIDATES = 6
    MAX_FRAME_PAYLOAD_SIZE = 65535

    def __init__(self, bluetooth_id="default", message_mode="iFacialMocap"):
        self.bluetooth_id = str(bluetooth_id)
        self.message_mode = message_mode

        # Kept as instance attributes for compatibility with the original code.
        self.NOTIFY_CHARACTERISTIC_UUID = self.__class__.NOTIFY_CHARACTERISTIC_UUID
        self.WRITE_CHARACTERISTIC_UUID = self.__class__.WRITE_CHARACTERISTIC_UUID
        self.SERVICE_UUID = self.generate_uuid_v5_from_string(
            self.SERVICE_NAMESPACE_UUID,
            self.bluetooth_id,
        )

        self.bluetooth_connection_mode_queue = queue.Queue()

        # Outgoing command queue retained under the original name.
        self.bluetooth_is_sending = False
        self.bluetooth_message_queue = queue.Queue()

        # The original code created bluetooth_message but did not consume it.
        # It is now the thread-safe queue of newly received decoded messages.
        self.bluetooth_message = queue.Queue(maxsize=200)
        self.received_message_queue = self.bluetooth_message

        self.bluetooth_thread = None
        self.bluetooth_client = None
        self.bluetooth_loop = None
        self.keepalive_future = None

        # The original spelling/casing is preserved for compatibility.
        self.recive_bluetooth_Mode = False
        self.recieved_message = None

        self.bluetooth_thread_lock = threading.Lock()
        self.encode_data_size = None
        self.bluetooth_received_data = bytearray()
        self.decompress_json_data = {}
        self.data_ready_event = threading.Event()

        self._lifecycle_lock = threading.RLock()
        self._queue_drain_lock = threading.Lock()
        self._loop_ready = threading.Event()
        self._loop_stopped = threading.Event()
        self._stopping = False

        # These asyncio objects are created inside the BLE event-loop thread.
        self._async_command_queue = None
        self._command_worker_task = None
        self._keepalive_task = None
        self._connect_lock = None
        self._send_lock = None

        self._stream_start_message = None
        self._streaming_requested = False
        self._connection_serial = 0
        self._stream_started_connection_serial = None

    # ------------------------------------------------------------------
    # Decode functions
    # ------------------------------------------------------------------

    def decode_blendshapes_data(self, data: bytes, index: int) -> Tuple[str, int]:
        blend_shapes = self.decompress_json_data.get("blendShapes", [])
        decoded = []

        for blend_shape_name in blend_shapes:
            if index >= len(data):
                break

            encoded_value = data[index]
            index += 1
            value = 0 if encoded_value == 255 else encoded_value - 100

            if self.message_mode == "iFacialMocap":
                value = max(0, value)
                decoded.append(f"{blend_shape_name}-{value}")
            else:
                decoded.append(f"{blend_shape_name}&{value}")

        return "|".join(decoded), index

    def decode_bones_data_0x02(self, data: bytes, index: int) -> Tuple[str, int]:
        bones_list = self.decompress_json_data.get("bones", [])
        decoded = []

        for bone_info in bones_list:
            if not isinstance(bone_info, (list, tuple)) or len(bone_info) != 2:
                continue

            bone_name, count = bone_info[0], int(bone_info[1])
            bone_values = []

            for value_index in range(count):
                if index + 1 >= len(data):
                    break

                int16_value = struct.unpack(">h", data[index:index + 2])[0]
                index += 2

                if bone_name == "head" and value_index >= 3:
                    value = int16_value / 10000.0
                else:
                    value = int16_value / 10.0
                bone_values.append(str(value))

            decoded.append(f"{bone_name}#{','.join(bone_values)}")

        return "|".join(decoded) + "|", index

    def decode_bones_data_0x03(self, data: bytes, index: int) -> Tuple[str, int]:
        bones_list = self.decompress_json_data.get("bones", [])
        decoded = []

        for bone_info in bones_list:
            if not isinstance(bone_info, (list, tuple)) or len(bone_info) != 2:
                continue

            bone_name, count = bone_info[0], int(bone_info[1])
            bone_values = []

            for _ in range(count):
                if index + 7 >= len(data):
                    break

                int64_value = struct.unpack(">q", data[index:index + 8])[0]
                index += 8
                bone_values.append(str(int64_value / 10_000_000.0))

            decoded.append(f"{bone_name}#{','.join(bone_values)}")

        return "|".join(decoded) + "|", index

    def values_decode_data(self, data: bytes, data_size: int) -> str:
        index = 0
        blend_shape_message_list = ""
        joint_message_list = ""
        payload = data[3:3 + data_size]

        while index < len(payload):
            identifier = payload[index]
            index += 1

            try:
                if identifier == 0x01:
                    blend_shape_message_list, index = self.decode_blendshapes_data(
                        payload,
                        index,
                    )
                elif identifier == 0x02:
                    joint_message_list, index = self.decode_bones_data_0x02(
                        payload,
                        index,
                    )
                elif identifier == 0x03:
                    joint_message_list, index = self.decode_bones_data_0x03(
                        payload,
                        index,
                    )
                else:
                    # Unknown identifier means that the frame cannot be decoded
                    # safely with the current schema.
                    return ""
            except Exception:
                traceback.print_exc()
                return ""

        return f"{blend_shape_message_list}={joint_message_list}"

    def decompress_zlib(self, compressed_data: bytes, data_size=None) -> Optional[str]:
        del data_size  # Retained only for API compatibility.
        try:
            return zlib.decompress(compressed_data).decode("utf-8")
        except Exception:
            traceback.print_exc()
            return None

    def generate_uuid_v5_from_string(self, namespace_uuid, input_string):
        return str(uuid.uuid5(uuid.UUID(str(namespace_uuid)), str(input_string)))

    # ------------------------------------------------------------------
    # Incoming BLE stream parsing
    # ------------------------------------------------------------------

    def _reset_receive_parser(self, clear_schema=False):
        with self.bluetooth_thread_lock:
            self.bluetooth_received_data.clear()
            self.encode_data_size = None
            if clear_schema:
                self.decompress_json_data = {}

    def _publish_received_message(self, message: str):
        if not message:
            return

        self.recieved_message = message

        try:
            self.bluetooth_message.put_nowait(message)
        except queue.Full:
            # Keep the newest real-time frames. Dropping one stale frame is more
            # useful than blocking Bleak's notification callback.
            try:
                self.bluetooth_message.get_nowait()
            except queue.Empty:
                pass
            try:
                self.bluetooth_message.put_nowait(message)
            except queue.Full:
                pass

        self.data_ready_event.set()

    def _handle_notification(
        self,
        sender: BleakGATTCharacteristic,
        data: bytearray,
    ):
        del sender

        if not self.recive_bluetooth_Mode:
            return

        completed_frames = []

        with self.bluetooth_thread_lock:
            self.bluetooth_received_data.extend(bytes(data))

            while True:
                if len(self.bluetooth_received_data) < 3:
                    self.encode_data_size = None
                    break

                packet_type = self.bluetooth_received_data[0]
                if packet_type not in (0x00, 0x01):
                    # Recover if a notification begins with stale/corrupt data.
                    del self.bluetooth_received_data[0]
                    self.encode_data_size = None
                    continue

                payload_size = struct.unpack(
                    ">H",
                    self.bluetooth_received_data[1:3],
                )[0]
                self.encode_data_size = payload_size

                if payload_size > self.MAX_FRAME_PAYLOAD_SIZE:
                    del self.bluetooth_received_data[0]
                    self.encode_data_size = None
                    continue

                frame_size = 3 + payload_size
                if len(self.bluetooth_received_data) < frame_size:
                    break

                frame = bytes(self.bluetooth_received_data[:frame_size])
                del self.bluetooth_received_data[:frame_size]
                self.encode_data_size = None
                completed_frames.append((packet_type, payload_size, frame))

        for packet_type, payload_size, frame in completed_frames:
            try:
                if packet_type == 0x00:
                    decoded_json = self.decompress_zlib(frame[3:], payload_size)
                    if decoded_json is None:
                        continue

                    schema = json.loads(decoded_json)
                    if isinstance(schema, dict):
                        self.decompress_json_data = schema
                elif packet_type == 0x01:
                    # The schema packet (0x00) defines BlendShape/bone order.
                    # Ignore value packets received before that schema arrives.
                    if not self.decompress_json_data:
                        continue
                    message = self.values_decode_data(frame, payload_size)
                    self._publish_received_message(message)
            except Exception:
                traceback.print_exc()

    def get_received_message(self, timeout=None) -> str:
        """Return one newly received frame.

        Unlike polling ``recieved_message``, each decoded BLE notification is
        returned at most once. ``queue.Empty`` is raised when a finite timeout
        expires.
        """

        message = self.bluetooth_message.get(timeout=timeout)
        if self.bluetooth_message.empty():
            self.data_ready_event.clear()
        return message

    # Correctly spelled convenience alias.
    get_recieved_message = get_received_message

    @property
    def received_message(self):
        return self.recieved_message

    def clear_received_messages(self):
        while True:
            try:
                self.bluetooth_message.get_nowait()
            except queue.Empty:
                break
        self.recieved_message = None
        self.data_ready_event.clear()
        with self.bluetooth_thread_lock:
            self.bluetooth_received_data.clear()
            self.encode_data_size = None

    # ------------------------------------------------------------------
    # Device discovery and connection
    # ------------------------------------------------------------------

    def _last_addr_path(self) -> str:
        safe_id = "".join(
            character if character.isalnum() or character in ("-", "_") else "_"
            for character in self.bluetooth_id
        )[:64]
        suffix = "" if safe_id == "default" else f"_{safe_id}"
        return os.path.join(
            os.path.expanduser("~"),
            f".ifm_last_ios_addr{suffix}.txt",
        )

    def _load_last_addr(self) -> Optional[str]:
        try:
            with open(self._last_addr_path(), "r", encoding="utf-8") as cache_file:
                value = cache_file.read().strip()
                return value or None
        except (OSError, UnicodeError):
            return None

    def _save_last_addr(self, address: str):
        try:
            with open(self._last_addr_path(), "w", encoding="utf-8") as cache_file:
                cache_file.write(address or "")
        except (OSError, UnicodeError):
            # Failure to persist the optimization must not break BLE.
            pass

    @classmethod
    def _name_matches(cls, device_name: Optional[str]) -> bool:
        normalized = (device_name or "").strip().lower()
        if not normalized:
            return False
        if normalized in cls.ACCEPTED_NAMES:
            return True
        return any(
            normalized.startswith(prefix)
            for prefix in cls.ACCEPTED_PREFIXES
        )

    @staticmethod
    def _entry_key(device) -> str:
        address = getattr(device, "address", None)
        if address:
            return str(address)
        return str(id(device))

    @staticmethod
    def _entry_name(device, advertisement_data) -> str:
        # Prefer the current advertisement name. On Windows, device.name can be
        # stale while AdvertisementData.local_name contains the latest iOS name.
        if advertisement_data is not None:
            local_name = getattr(advertisement_data, "local_name", None)
            if local_name:
                return str(local_name)

        device_name = getattr(device, "name", None)
        if device_name:
            return str(device_name)

        return ""

    def _entry_advertises_target_service(self, advertisement_data) -> bool:
        if advertisement_data is None:
            return False

        service_uuids = getattr(advertisement_data, "service_uuids", None) or []
        target = self.SERVICE_UUID.lower()
        return any(str(service_uuid).lower() == target for service_uuid in service_uuids)

    async def _discover_entries(
        self,
        timeout_seconds: float,
        service_uuids: Optional[Sequence[str]] = None,
    ) -> List[Tuple[Any, Any]]:
        kwargs = {}
        if service_uuids:
            kwargs["service_uuids"] = list(service_uuids)

        try:
            result = await BleakScanner.discover(
                timeout=timeout_seconds,
                return_adv=True,
                **kwargs,
            )
        except TypeError:
            # Compatibility with older Bleak versions that do not support
            # return_adv on discover().
            result = await BleakScanner.discover(
                timeout=timeout_seconds,
                **kwargs,
            )

        entries = []
        if isinstance(result, dict):
            for value in result.values():
                if isinstance(value, tuple) and len(value) >= 2:
                    entries.append((value[0], value[1]))
                else:
                    entries.append((value, None))
        else:
            entries.extend((device, None) for device in (result or []))

        unique_entries = []
        seen = set()
        for device, advertisement_data in entries:
            key = self._entry_key(device)
            if key in seen:
                continue
            seen.add(key)
            unique_entries.append((device, advertisement_data))
        return unique_entries

    def _order_candidates(
        self,
        entries: Sequence[Tuple[Any, Any]],
        include_unknown: bool,
    ) -> List[Any]:
        last_address = self._load_last_addr()
        ordered_entries = []
        added = set()

        def add_matching(predicate):
            for device, advertisement_data in entries:
                key = self._entry_key(device)
                if key in added:
                    continue
                if predicate(device, advertisement_data):
                    added.add(key)
                    ordered_entries.append((device, advertisement_data))

        if last_address:
            normalized_last_address = last_address.lower()
            add_matching(
                lambda device, _adv: str(
                    getattr(device, "address", "")
                ).lower() == normalized_last_address
            )

        add_matching(
            lambda _device, adv: self._entry_advertises_target_service(adv)
        )
        add_matching(
            lambda device, adv: self._name_matches(
                self._entry_name(device, adv)
            )
        )

        if include_unknown:
            unknown_entries = []
            for device, advertisement_data in entries:
                key = self._entry_key(device)
                if key in added:
                    continue

                # The target iPhone sometimes appears as name=None on Windows.
                # Skip clearly unrelated named devices such as headphones.
                name = self._entry_name(device, advertisement_data)
                if name:
                    continue

                rssi = getattr(advertisement_data, "rssi", -999)
                unknown_entries.append((rssi, device, advertisement_data))

            # Try the nearest unnamed devices first and cap the fallback count.
            unknown_entries.sort(key=lambda entry: entry[0], reverse=True)
            for _rssi, device, advertisement_data in unknown_entries[
                :self.MAX_UNKNOWN_CANDIDATES
            ]:
                key = self._entry_key(device)
                added.add(key)
                ordered_entries.append((device, advertisement_data))

        return [device for device, _advertisement_data in ordered_entries]

    async def _scan_candidates(self) -> List[Any]:
        # iOS advertises the UUIDv5 service derived from bluetooth_id. This is
        # the fast and safe path and avoids connecting to unrelated BLE devices.
        try:
            targeted_entries = await self._discover_entries(
                self.TARGETED_SCAN_SECONDS,
                service_uuids=[self.SERVICE_UUID],
            )
        except Exception as error:
            print(f"Targeted Bluetooth scan failed; using fallback scan: {error}")
            targeted_entries = []

        if targeted_entries:
            candidates = self._order_candidates(
                targeted_entries,
                include_unknown=True,
            )

            # A scan filtered by SERVICE_UUID has already identified these as
            # target devices. Older Bleak versions may not return advertisement
            # metadata, so include any remaining filtered entries even when the
            # device has a generic/non-empty Windows name.
            candidate_keys = {self._entry_key(device) for device in candidates}
            for device, _advertisement_data in targeted_entries:
                key = self._entry_key(device)
                if key not in candidate_keys:
                    candidates.append(device)
                    candidate_keys.add(key)

            if candidates:
                return candidates

        try:
            fallback_entries = await self._discover_entries(
                self.FALLBACK_SCAN_SECONDS,
            )
        except Exception as error:
            print(f"Bluetooth scan failed: {error}")
            return []

        return self._order_candidates(
            fallback_entries,
            include_unknown=True,
        )

    async def _has_required_characteristics(self, client) -> bool:
        needed_notify = self.NOTIFY_CHARACTERISTIC_UUID.lower()
        needed_write = self.WRITE_CHARACTERISTIC_UUID.lower()

        services = None
        for _ in range(20):
            try:
                services = client.services
                if services:
                    break
            except Exception:
                pass
            await asyncio.sleep(0.1)

        if not services:
            return False

        try:
            has_notify = (
                services.get_characteristic(self.NOTIFY_CHARACTERISTIC_UUID)
                is not None
            )
            has_write = (
                services.get_characteristic(self.WRITE_CHARACTERISTIC_UUID)
                is not None
            )
            if has_notify and has_write:
                return True
        except Exception:
            pass

        try:
            discovered_characteristics = set()
            for service in services:
                for characteristic in service.characteristics:
                    discovered_characteristics.add(str(characteristic.uuid).lower())
            return (
                needed_notify in discovered_characteristics
                and needed_write in discovered_characteristics
            )
        except Exception:
            return False

    @staticmethod
    def _client_is_connected(client) -> bool:
        if client is None:
            return False
        try:
            return bool(client.is_connected)
        except Exception:
            return False

    def _on_client_disconnected(self, client):
        if self.bluetooth_client is client:
            self.bluetooth_client = None
            self._stream_started_connection_serial = None
            try:
                self.bluetooth_connection_mode_queue.put_nowait("disconnected")
            except queue.Full:
                pass

    async def _disconnect_client(self, client=None):
        client_to_disconnect = client or self.bluetooth_client
        if client_to_disconnect is None:
            return

        if self.bluetooth_client is client_to_disconnect:
            self.bluetooth_client = None
            self._stream_started_connection_serial = None

        if self._client_is_connected(client_to_disconnect):
            try:
                await client_to_disconnect.stop_notify(
                    self.NOTIFY_CHARACTERISTIC_UUID
                )
            except Exception:
                pass
            try:
                await client_to_disconnect.disconnect()
            except Exception:
                pass

    async def _ensure_connected(self) -> bool:
        if self._stopping:
            return False
        if self._client_is_connected(self.bluetooth_client):
            return True

        if self._connect_lock is None:
            self._connect_lock = asyncio.Lock()

        async with self._connect_lock:
            if self._stopping:
                return False
            if self._client_is_connected(self.bluetooth_client):
                return True

            try:
                self.bluetooth_connection_mode_queue.put_nowait("connecting")
            except queue.Full:
                pass

            candidates = await self._scan_candidates()
            if not candidates:
                print("No iFacialMocap/Facemotion3d Bluetooth candidates found.")
                return False

            for candidate_index, device in enumerate(candidates):
                if self._stopping:
                    return False

                name = getattr(device, "name", None)
                address = getattr(device, "address", "unknown")
                print(f"Connecting to: {name} ({address})")

                timeout_seconds = (
                    self.FIRST_CONNECT_TIMEOUT_SECONDS
                    if candidate_index == 0
                    else self.OTHER_CONNECT_TIMEOUT_SECONDS
                )

                client = BleakClient(
                    device,
                    timeout=timeout_seconds,
                    disconnected_callback=self._on_client_disconnected,
                )
                adopted = False

                try:
                    await asyncio.wait_for(
                        client.connect(),
                        timeout=timeout_seconds + 2.0,
                    )

                    if not self._client_is_connected(client):
                        continue

                    if not await self._has_required_characteristics(client):
                        print(
                            "Connected but target characteristics were not "
                            "found. Trying next candidate..."
                        )
                        continue

                    self._reset_receive_parser(clear_schema=True)
                    self.bluetooth_client = client

                    try:
                        await client.start_notify(
                            self.NOTIFY_CHARACTERISTIC_UUID,
                            self._handle_notification,
                        )
                        if not self._client_is_connected(client):
                            raise ConnectionError(
                                "Disconnected while enabling BLE notifications."
                            )
                    except Exception:
                        self.bluetooth_client = None
                        raise

                    adopted = True
                    self._connection_serial += 1
                    self._stream_started_connection_serial = None
                    self._save_last_addr(str(address))

                    print("Successfully connected to Bluetooth.")
                    try:
                        self.bluetooth_connection_mode_queue.put_nowait(
                            "connected"
                        )
                    except queue.Full:
                        pass
                    return True

                except asyncio.CancelledError:
                    raise
                except Exception as error:
                    print(f"Connection attempt failed: {error}")
                finally:
                    if not adopted:
                        await self._disconnect_client(client)

            print("No device with the required characteristics was found.")
            return False

    # ------------------------------------------------------------------
    # Sending and background workers
    # ------------------------------------------------------------------

    @staticmethod
    def _is_stop_streaming_command(message: str) -> bool:
        return message.startswith("StopStreaming")

    @staticmethod
    def _is_stream_start_command(message: str) -> bool:
        return message.startswith("iFacialMocap_")

    async def _write_protocol_message(
        self,
        message: str,
        skip_duplicate_stream_start=False,
    ) -> bool:
        if self._send_lock is None:
            self._send_lock = asyncio.Lock()

        async with self._send_lock:
            client = self.bluetooth_client
            if not self._client_is_connected(client):
                return False

            if (
                skip_duplicate_stream_start
                and self._is_stream_start_command(message)
                and self._stream_started_connection_serial
                == self._connection_serial
            ):
                return True

            try:
                await client.write_gatt_char(
                    self.WRITE_CHARACTERISTIC_UUID,
                    bytearray(message, "utf-8"),
                    response=True,
                )
            except asyncio.CancelledError:
                raise
            except Exception as error:
                print(f"Error sending message: {error}")
                traceback.print_exc()
                await self._disconnect_client(client)
                return False

            if self._is_stop_streaming_command(message):
                self._stream_started_connection_serial = None
            elif self._is_stream_start_command(message):
                self._stream_started_connection_serial = self._connection_serial

            return True

    async def bluetooth_send_message(self, message):
        """Send one message over the existing connection.

        This method no longer clears the receive buffer, so sending a command
        cannot destroy a partially received BLE frame.
        """

        return await self._write_protocol_message(str(message))

    async def bluetooth_recieve_realtime_start(self, message):
        """Compatibility wrapper for the original async method name."""

        if not await self._ensure_connected():
            return False
        return await self._write_protocol_message(
            str(message),
            skip_duplicate_stream_start=True,
        )

    async def _send_command_with_retry(self, message: str) -> bool:
        for attempt in range(2):
            if self._stopping:
                return False

            if await self._ensure_connected():
                sent = await self._write_protocol_message(
                    message,
                    skip_duplicate_stream_start=True,
                )
                if sent:
                    print(f"Successfully sent message: {message}")
                    return True

            if attempt == 0 and not self._stopping:
                await asyncio.sleep(0.75)

        print(f"Could not send message: {message}")
        return False

    async def _command_worker(self):
        while True:
            message = await self._async_command_queue.get()
            try:
                await self._send_command_with_retry(message)
            except asyncio.CancelledError:
                raise
            except Exception:
                traceback.print_exc()
            finally:
                self._async_command_queue.task_done()

    async def _keepalive_worker(self):
        reconnect_delay = 1.0

        while True:
            try:
                if (
                    self._streaming_requested
                    and self.recive_bluetooth_Mode
                    and self._stream_start_message
                ):
                    if not self._client_is_connected(self.bluetooth_client):
                        connected = await self._ensure_connected()
                        if not connected:
                            await asyncio.sleep(reconnect_delay)
                            reconnect_delay = min(reconnect_delay * 2.0, 10.0)
                            continue

                    reconnect_delay = 1.0
                    if (
                        self._stream_started_connection_serial
                        != self._connection_serial
                    ):
                        await self._write_protocol_message(
                            self._stream_start_message,
                            skip_duplicate_stream_start=True,
                        )

                await asyncio.sleep(1.0)
            except asyncio.CancelledError:
                raise
            except Exception:
                traceback.print_exc()
                await asyncio.sleep(1.0)

    def _loop_thread_main(self):
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)

        with self._lifecycle_lock:
            self.bluetooth_loop = loop
            self._async_command_queue = asyncio.Queue()
            self._connect_lock = asyncio.Lock()
            self._send_lock = asyncio.Lock()
            self._command_worker_task = loop.create_task(
                self._command_worker()
            )
            self._keepalive_task = loop.create_task(
                self._keepalive_worker()
            )
            self.keepalive_future = self._keepalive_task
            self._loop_ready.set()

        try:
            loop.run_forever()
        finally:
            try:
                loop.run_until_complete(self._shutdown_async())
            except Exception:
                traceback.print_exc()

            try:
                pending = [
                    task
                    for task in asyncio.all_tasks(loop)
                    if not task.done()
                ]
                for task in pending:
                    task.cancel()
                if pending:
                    loop.run_until_complete(
                        asyncio.gather(*pending, return_exceptions=True)
                    )
            except Exception:
                traceback.print_exc()

            loop.close()

            self._streaming_requested = False
            self.recive_bluetooth_Mode = False
            self._reset_receive_parser(clear_schema=True)

            with self._lifecycle_lock:
                self.bluetooth_loop = None
                self.bluetooth_client = None
                self._async_command_queue = None
                self._connect_lock = None
                self._send_lock = None
                self._command_worker_task = None
                self._keepalive_task = None
                self.keepalive_future = None
                if self.bluetooth_thread is threading.current_thread():
                    self.bluetooth_thread = None
                self._stopping = False
                self._loop_ready.clear()
                self._loop_stopped.set()

    def _start_loop_thread(self):
        with self._lifecycle_lock:
            if (
                self.bluetooth_thread is not None
                and self.bluetooth_thread.is_alive()
            ):
                # The thread may still be creating its event loop. Do not start
                # a second BLE loop; wait for _loop_ready below.
                thread = self.bluetooth_thread
            else:
                self._stopping = False
                self._loop_ready.clear()
                self._loop_stopped.clear()
                thread = threading.Thread(
                    target=self._loop_thread_main,
                    name="iFM-FM-BLE-EventLoop",
                    daemon=True,
                )
                self.bluetooth_thread = thread
                thread.start()

        if not self._loop_ready.wait(timeout=5.0):
            raise RuntimeError("Bluetooth event loop failed to start.")

    def _enqueue_async_command(self, message: str):
        self._start_loop_thread()

        loop = self.bluetooth_loop
        command_queue = self._async_command_queue
        if loop is None or command_queue is None or not loop.is_running():
            raise RuntimeError("Bluetooth event loop is not running.")

        loop.call_soon_threadsafe(command_queue.put_nowait, message)

    def bluetooth_send_message_trigger(self, message):
        """Queue a Bluetooth command without blocking the caller."""

        message = str(message)

        if self._is_stop_streaming_command(message):
            self._streaming_requested = False
            self.recive_bluetooth_Mode = False
        elif self._is_stream_start_command(message):
            self._stream_start_message = message
            self._streaming_requested = True
            self.recive_bluetooth_Mode = True
            self.clear_received_messages()
            self.decompress_json_data = {}

        self.bluetooth_message_queue.put(message)
        self.process_message_queue()

    def process_message_queue(self):
        """Move pending legacy queue entries to the asyncio command worker.

        The original recursive, blocking implementation has been replaced by a
        non-recursive queue drain.
        """

        # The critical section is short. Blocking here avoids a race where a
        # second caller appends a command just after the first drain observes an
        # empty queue, which could otherwise leave that command stranded.
        self._queue_drain_lock.acquire()

        self.bluetooth_is_sending = True
        try:
            while True:
                try:
                    message = self.bluetooth_message_queue.get_nowait()
                except queue.Empty:
                    break

                try:
                    self._enqueue_async_command(str(message))
                except Exception:
                    # Preserve the command if the event loop could not start.
                    self.bluetooth_message_queue.put(message)
                    raise
        finally:
            self.bluetooth_is_sending = False
            self._queue_drain_lock.release()

    def run_bluetooth_loop(self, message):
        """Compatibility wrapper retained for existing callers."""

        self.bluetooth_send_message_trigger(message)

    async def keep_bluetooth_alive(self, message):
        """Compatibility wrapper for code that awaited the old method."""

        if message and self._is_stream_start_command(str(message)):
            self._stream_start_message = str(message)
            self._streaming_requested = True
            self.recive_bluetooth_Mode = True
        await self._keepalive_worker()

    def wait_for_all_commands(self, timeout=5.0) -> bool:
        # First flush commands that may still be in the compatibility queue.
        self.process_message_queue()

        loop = self.bluetooth_loop
        command_queue = self._async_command_queue
        if loop is None or command_queue is None or not loop.is_running():
            return self.bluetooth_message_queue.empty()

        async def wait_for_queue_barrier():
            # Let previously scheduled put_nowait callbacks execute before join.
            await asyncio.sleep(0)
            await command_queue.join()

        future = asyncio.run_coroutine_threadsafe(
            wait_for_queue_barrier(),
            loop,
        )
        try:
            future.result(timeout=timeout)
            return True
        except FuturesTimeoutError:
            future.cancel()
            return False
        except FuturesCancelledError:
            return False

    # ------------------------------------------------------------------
    # Shutdown and restart
    # ------------------------------------------------------------------

    async def stop_async_tasks(self):
        current_task = asyncio.current_task()
        tasks = [self._command_worker_task, self._keepalive_task]

        for task in tasks:
            if task is not None and task is not current_task and not task.done():
                task.cancel()

        tasks_to_wait = [
            task
            for task in tasks
            if task is not None and task is not current_task
        ]
        if tasks_to_wait:
            await asyncio.gather(*tasks_to_wait, return_exceptions=True)

        self._command_worker_task = None
        self._keepalive_task = None
        self.keepalive_future = None

    async def stop_notifications_and_cleanup(self):
        await self._disconnect_client()

    async def _shutdown_async(self):
        await self.stop_async_tasks()
        await self.stop_notifications_and_cleanup()

    def _clear_queue(self, target_queue):
        while True:
            try:
                target_queue.get_nowait()
            except queue.Empty:
                return

    def bluetooth_stop(self):
        """Stop BLE cleanly. The same instance can be started again later."""

        self._streaming_requested = False
        self.recive_bluetooth_Mode = False
        self._stopping = True

        with self._lifecycle_lock:
            loop = self.bluetooth_loop
            thread = self.bluetooth_thread

        if loop is not None and loop.is_running():
            if thread is threading.current_thread():
                async def stop_from_loop_thread():
                    await self._shutdown_async()
                    loop.stop()

                loop.create_task(stop_from_loop_thread())
                return

            future = asyncio.run_coroutine_threadsafe(
                self._shutdown_async(),
                loop,
            )
            try:
                future.result(timeout=8.0)
            except FuturesTimeoutError:
                print("Bluetooth cleanup timed out; forcing event loop stop.")
                future.cancel()
            except FuturesCancelledError:
                pass
            except Exception:
                traceback.print_exc()

            try:
                loop.call_soon_threadsafe(loop.stop)
            except RuntimeError:
                pass

        if (
            thread is not None
            and thread is not threading.current_thread()
            and thread.is_alive()
        ):
            thread.join(timeout=5.0)
            if thread.is_alive():
                print("Bluetooth event-loop thread did not stop within 5 seconds.")

        with self._lifecycle_lock:
            if self.bluetooth_thread is thread and (
                thread is None or not thread.is_alive()
            ):
                self.bluetooth_thread = None

        self._clear_queue(self.bluetooth_message_queue)
        self._clear_queue(self.bluetooth_message)
        self._reset_receive_parser(clear_schema=True)
        self.recieved_message = None
        self.data_ready_event.clear()
        self._stream_start_message = None
        self._stream_started_connection_serial = None
        self.bluetooth_client = None

        if thread is None or not thread.is_alive():
            self._stopping = False

        try:
            self.bluetooth_connection_mode_queue.put_nowait("stopped")
        except queue.Full:
            pass


# Correctly spelled optional alias for new code. Existing code can continue to
# use ifm_and_fm_bluetooh_functions unchanged.
IFMAndFMBluetoothFunctions = ifm_and_fm_bluetooh_functions


if __name__ == "__main__":
    bluetooth_id = "default"
    start_streaming_message = (
        "iFacialMocap_sahuasouryya9218sauhuiayeta91555dy3719"
    )
    bluetooth_instance = ifm_and_fm_bluetooh_functions(bluetooth_id)

    def print_new_messages(label, count=5, timeout_per_message=15.0):
        for _ in range(count):
            try:
                message = bluetooth_instance.get_received_message(
                    timeout=timeout_per_message
                )
            except queue.Empty:
                print(f"Timed out waiting for a new {label} message.")
                return False

            print(f"{label} recieved message: ", message)
            print()
        return True

    try:
        # First session: iFacialMocap format.
        bluetooth_instance.message_mode = "iFacialMocap"
        bluetooth_instance.bluetooth_send_message_trigger(
            start_streaming_message
        )
        print()
        print_new_messages("iFM")

        bluetooth_instance.bluetooth_send_message_trigger("StopStreaming")
        bluetooth_instance.wait_for_all_commands(timeout=3.0)

        # Second session: Facemotion3d format. Set the mode before sending.
        bluetooth_instance.message_mode = "Facemotion3d"
        bluetooth_instance.bluetooth_send_message_trigger(
            start_streaming_message
        )
        print_new_messages("FM")

        bluetooth_instance.bluetooth_send_message_trigger("StopStreaming")
        bluetooth_instance.wait_for_all_commands(timeout=3.0)

    except KeyboardInterrupt:
        print("Bluetooth sample interrupted.")
    finally:
        bluetooth_instance.bluetooth_stop()


```
