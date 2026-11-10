package com.wusla.hams.core

import java.io.File

object CpuFeatures {
    /** Whether every core reports ARMv8.2 dot-product ("asimddp") and fp16 ("asimdhp") support. */
    fun hasDotProduct(cpuinfo: String? = read()): Boolean {
        val lines = cpuinfo?.lineSequence()?.filter { it.startsWith("Features") }?.toList().orEmpty()
        if (lines.isEmpty()) return false
        return lines.all { line ->
            val features = line.substringAfter(':').trim().split(Regex("\\s+")).toSet()
            "asimddp" in features && "asimdhp" in features
        }
    }

    private fun read(): String? = try {
        File("/proc/cpuinfo").readText()
    } catch (_: Exception) {
        null
    }
}
