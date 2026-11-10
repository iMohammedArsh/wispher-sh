package com.wusla.hams

import com.wusla.hams.core.CpuFeatures
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class CpuFeaturesTest {
    @Test
    fun modernCoresUseTheFastBuild() {
        val info = "processor : 0\nFeatures : fp asimd evtstrm aes asimdhp cpuid asimdrdm asimddp\nprocessor : 1\nFeatures : fp asimd asimdhp asimddp\n"
        assertTrue(CpuFeatures.hasDotProduct(info))
    }

    @Test
    fun anyOldCoreMeansTheCompatibleBuild() {
        // big.LITTLE with an A53 little core: work can land on it, so play safe.
        val info = "Features : fp asimd asimdhp asimddp\nFeatures : fp asimd evtstrm aes crc32\n"
        assertFalse(CpuFeatures.hasDotProduct(info))
        assertFalse(CpuFeatures.hasDotProduct(""))
        assertFalse(CpuFeatures.hasDotProduct(null))
    }
}
