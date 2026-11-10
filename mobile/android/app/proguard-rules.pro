# JNA and the UniFFI bindings are called reflectively from native code.
-keep class com.sun.jna.** { *; }
-keepclassmembers class * extends com.sun.jna.** { public *; }
-keep class com.wusla.hams.core.** { *; }
-dontwarn java.awt.**
-dontwarn com.sun.jna.**
