package com.taskmod.app

import java.net.Inet4Address
import java.net.InetAddress
import java.net.NetworkInterface
import java.net.Socket
import java.net.DatagramSocket
import java.net.DatagramPacket
import java.net.InetSocketAddress

object NetworkHelper {

    data class NetInfo(val name: String, val ip: String, val type: String)
    data class DiscoveredServer(val ip: String, val port: Int, val type: String)

    fun getAllIps(): List<NetInfo> {
        val result = mutableListOf<NetInfo>()
        try {
            val interfaces = NetworkInterface.getNetworkInterfaces()
            while (interfaces.hasMoreElements()) {
                val ni = interfaces.nextElement()
                if (ni.isLoopback || !ni.isUp) continue
                val addresses = ni.inetAddresses
                while (addresses.hasMoreElements()) {
                    val addr = addresses.nextElement()
                    if (!addr.isLoopbackAddress && addr is Inet4Address) {
                        val ip = addr.hostAddress ?: continue
                        val type = when {
                            ni.name.startsWith("wlan") -> "WiFi"
                            ni.name.startsWith("eth") -> "以太网"
                            ni.name.startsWith("rmnet") -> "蜂窝"
                            ni.name.startsWith("tun") -> "VPN"
                            else -> ni.displayName ?: ni.name
                        }
                        result.add(NetInfo(ni.name, ip, type))
                    }
                }
            }
        } catch (e: Exception) {
        }
        return result
    }

    fun getLocalIpAddress(): String {
        val ips = getAllIps()
        return ips.firstOrNull { it.type == "WiFi" }?.ip
            ?: ips.firstOrNull { it.type == "以太网" }?.ip
            ?: ips.firstOrNull()?.ip
            ?: "0.0.0.0"
    }

    fun getLanSubnet(): String {
        val ip = getLocalIpAddress()
        val parts = ip.split(".")
        if (parts.size == 4) {
            return "${parts[0]}.${parts[1]}.${parts[2]}."
        }
        return ""
    }

    fun scanLanForServer(port: Int, timeoutMs: Int = 500): List<DiscoveredServer> {
        val results = mutableListOf<DiscoveredServer>()
        val subnet = getLanSubnet()
        if (subnet.isEmpty()) return results

        val checked = mutableListOf<Boolean>()
        val locks = mutableListOf<Any>()
        for (_ in 1..254) {
            checked.add(false)
            locks.add(Any())
        }

        val threads = mutableListOf<Thread>()
        for (i in 1..254) {
            val ip = "${subnet}$i"
            val index = i - 1
            threads.add(Thread {
                try {
                    Socket().use { socket ->
                        socket.connect(InetSocketAddress(ip, port), timeoutMs)
                        synchronized(locks[index]) {
                            if (!checked[index]) {
                                checked[index] = true
                                results.add(DiscoveredServer(ip, port, "LAN"))
                            }
                        }
                    }
                } catch (e: Exception) {
                }
            })
        }

        threads.forEach { it.start() }
        threads.forEach {
            try {
                it.join(timeoutMs.toLong() + 1000)
            } catch (e: InterruptedException) {
            }
        }

        return results
    }

    fun discoverViaBroadcast(port: Int): List<DiscoveredServer> {
        val results = mutableListOf<DiscoveredServer>()
        try {
            DatagramSocket().use { socket ->
                socket.soTimeout = 3000
                socket.broadcast = true

                val msg = "TASKMOD_DISCOVERY".toByteArray()
                val broadcastAddr = InetAddress.getByName("255.255.255.255")
                socket.send(DatagramPacket(msg, msg.size, broadcastAddr, port))

                val buf = ByteArray(256)
                val packet = DatagramPacket(buf, buf.size)
                try {
                    while (true) {
                        socket.receive(packet)
                        val response = String(packet.data, 0, packet.length)
                        if (response.startsWith("TASKMOD_SERVER")) {
                            val ip = packet.address.hostAddress
                            if (ip != null && !results.any { it.ip == ip }) {
                                results.add(DiscoveredServer(ip, port, "Broadcast"))
                            }
                        }
                    }
                } catch (e: java.net.SocketTimeoutException) {
                }
            }
        } catch (e: Exception) {
        }
        return results
    }

    fun isReachable(ip: String, port: Int, timeoutMs: Int = 1000): Boolean {
        return try {
            Socket().use { socket ->
                socket.connect(InetSocketAddress(ip, port), timeoutMs)
                true
            }
        } catch (e: Exception) {
            false
        }
    }
}