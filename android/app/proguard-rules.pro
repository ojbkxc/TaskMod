# ========== Kotlin & Coroutines ==========
-keepnames class kotlinx.coroutines.internal.MainDispatcherFactory {}
-keepnames class kotlinx.coroutines.CoroutineExceptionHandler {}
-keepclassmembers class kotlinx.coroutines.** {
    volatile <fields>;
}
-dontwarn kotlinx.coroutines.**
-keep class kotlinx.coroutines.** { *; }

# ========== Gson ==========
-keepattributes Signature
-keepattributes *Annotation*
-dontwarn sun.misc.**
-keep class com.google.gson.** { *; }
-keep class * extends com.google.gson.TypeAdapter
-keep class * implements com.google.gson.TypeAdapterFactory
-keep class * implements com.google.gson.JsonSerializer
-keep class * implements com.google.gson.JsonDeserializer

# ========== OkHttp ==========
-dontwarn okhttp3.**
-dontwarn okio.**
-keep class okhttp3.** { *; }
-keep class okio.** { *; }

# ========== TaskMod 数据类 ==========
-keep class com.taskmod.app.UpdateInfo { *; }
-keep class com.taskmod.app.ConfigManager$AppConfig { *; }
-keep class com.taskmod.app.NetworkHelper$DiscoveredServer { *; }
-keep class com.taskmod.app.NetworkHelper$NetInfo { *; }
-keep class com.taskmod.app.RootHelper$RootResult { *; }
-keep class com.taskmod.app.ServerManager$ServerState { *; }
-keep class com.taskmod.app.DaemonManager$DaemonStatus { *; }
-keep class com.taskmod.app.DaemonManager$OperationResult { *; }

# ========== AndroidX & Material ==========
-dontwarn com.google.android.material.**
-keep class com.google.android.material.** { *; }
-dontwarn androidx.**
-keep class androidx.** { *; }

# ========== 通用规则 ==========
-keepclassmembers,allowobfuscation class * {
    @com.google.gson.annotations.SerializedName <fields>;
}
-keepattributes SourceFile,LineNumberTable
-keepattributes EnclosingMethod
-keepattributes InnerClasses

# ========== 保持 R 类 ==========
-keepclassmembers class **.R$* {
    public static <fields>;
}

# ========== 保持 Application 和 Service ==========
-keep class com.taskmod.app.TaskModApp { *; }
-keep class com.taskmod.app.TaskModService { *; }
-keep class com.taskmod.app.BootReceiver { *; }
-keep class com.taskmod.app.UpdateReceiver { *; }

# ========== WebView ==========
-keepclassmembers class * extends android.webkit.WebView {
    public <init>(android.content.Context);
    public <init>(android.content.Context, android.util.AttributeSet);
    public <init>(android.content.Context, android.util.AttributeSet, int);
}
-keep class android.webkit.JavascriptInterface { *; }
-keepclassmembers class * {
    @android.webkit.JavascriptInterface <methods>;
}

# ========== 保持枚举 ==========
-keepclassmembers enum * {
    public static **[] values();
    public static ** valueOf(java.lang.String);
}

# ========== 保持序列化相关 ==========
-keepclassmembers class * implements java.io.Serializable {
    static final long serialVersionUID;
    private static final java.io.ObjectStreamField[] serialPersistentFields;
    private void writeObject(java.io.ObjectOutputStream);
    private void readObject(java.io.ObjectInputStream);
    java.lang.Object writeReplace();
    java.lang.Object readResolve();
}