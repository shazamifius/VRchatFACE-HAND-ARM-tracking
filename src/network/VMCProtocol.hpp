#pragma once

#include "../core/MathUtils.hpp"
#include <cstring>
#include <string>
#include <vector>


namespace Network {

class VMCProtocol {
public:
  // OSC Packet construction helper
  // VMC Bone Posiiton: /VMC/Ext/Bone/Pos (s)(sfffffff)
  // string: name, string: boneName, float: pos x, y, z, rot x, y, z, w
  static std::vector<char> PackBonePos(const std::string &bone_name,
                                       const Core::Vector3 &pos,
                                       const Core::Quaternion &rot) {
    std::vector<char> packet;

    // Address Pattern
    AppendString(packet, "/VMC/Ext/Bone/Pos");

    // Type Tag String
    AppendString(packet, ",ssfffffff");

    // Arguments
    AppendString(packet, "root"); // Model name (arbitrary for now)
    AppendString(packet, bone_name);
    AppendFloat(packet, pos.x);
    AppendFloat(packet, pos.y);
    AppendFloat(packet, pos.z);
    AppendFloat(packet, rot.x);
    AppendFloat(packet, rot.y);
    AppendFloat(packet, rot.z);
    AppendFloat(packet, rot.w);

    return packet;
  }

private:
  static void AppendString(std::vector<char> &packet, const std::string &s) {
    packet.insert(packet.end(), s.begin(), s.end());
    packet.push_back('\0');
    // Pad to 4 bytes
    while (packet.size() % 4 != 0) {
      packet.push_back('\0');
    }
  }

  static void AppendFloat(std::vector<char> &packet, float f) {
    // OSC requires Big Endian for floats. Windows is Little Endian.
    // Need to swap bytes.
    uint32_t i;
    std::memcpy(&i, &f, 4);
    i = htonl(i); // Host to Network Long (Big Endian)

    char bytes[4];
    std::memcpy(bytes, &i, 4);
    packet.insert(packet.end(), bytes, bytes + 4);
  }
};

} // namespace Network
