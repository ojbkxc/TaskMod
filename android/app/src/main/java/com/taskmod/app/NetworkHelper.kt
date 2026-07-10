package com.taskmod.app

import java.net.Inet4Address
import java.net.NetworkInterface

object NetworkHelper {

    data class NetInfo(val name: String, val ip: String, val type: String)

    /** 获取所有可用的 IPv4 地址 */
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
            // ignore
        }
        return result
    }

    /** 获取首选 LAN IP（WiFi 优先） */
    fun getLocalIpAddress(): String {
        val ips = getAllIps()
        return ips.firstOrNull { it.type == "WiFi" }?.ip
            ?: ips.firstOrNull { it.type == "以太网" }?.ip
            ?: ips.firstOrNull()?.ip
            ?: "0.0.0.0"
    }
}
