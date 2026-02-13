#include "../network/OSCClient.hpp"
#include "../network/VMCProtocol.hpp"
#include <gtest/gtest.h>


TEST(NetworkTest, VMCSerialization) {
  Core::Vector3 pos(1.0f, 2.0f, 3.0f);
  Core::Quaternion rot(0.0f, 0.0f, 0.0f, 1.0f);

  std::vector<char> packet =
      Network::VMCProtocol::PackBonePos("Hips", pos, rot);

  // OSC string padding check
  // /VMC/Ext/Bone/Pos (17 chars) -> padded to 20 bytes
  // ,ssfffffff (10 chars) -> padded to 12 bytes
  // "root" (4 chars) -> padded to 4 bytes
  // "Hips" (4 chars) -> padded to 4 bytes
  // 7 floats = 28 bytes
  // Total: 20 + 12 + 4 + 4 + 28 = 68 bytes

  EXPECT_GE(packet.size(), 60);

  // Check address pattern start
  const char *addr = "/VMC/Ext/Bone/Pos";
  EXPECT_EQ(std::memcmp(packet.data(), addr, strlen(addr)), 0);
}

TEST(NetworkTest, OSCClientInit) {
  // Just verify constructor doesn't crash on standard ports
  EXPECT_NO_THROW({ Network::OSCClient client("127.0.0.1", 39539); });
}
