package wit_trinity_module;

import java.nio.charset.StandardCharsets;
import java.util.ArrayList;

import org.teavm.interop.Memory;
import org.teavm.interop.Address;
import org.teavm.interop.Import;
import org.teavm.interop.Export;

public final class Log {
    private Log() {}
    
    @Import(name = "trace", module = "log")
    private static native void wasmImportTrace(int p0, int p1);
    
    public static void trace(String s) {
        byte[] bytes = (s).getBytes(StandardCharsets.UTF_8);
        wasmImportTrace(Address.ofData(bytes).toInt(), bytes.length);
        
    }
    @Import(name = "debug", module = "log")
    private static native void wasmImportDebug(int p0, int p1);
    
    public static void debug(String s) {
        byte[] bytes = (s).getBytes(StandardCharsets.UTF_8);
        wasmImportDebug(Address.ofData(bytes).toInt(), bytes.length);
        
    }
    @Import(name = "info", module = "log")
    private static native void wasmImportInfo(int p0, int p1);
    
    public static void info(String s) {
        byte[] bytes = (s).getBytes(StandardCharsets.UTF_8);
        wasmImportInfo(Address.ofData(bytes).toInt(), bytes.length);
        
    }
    @Import(name = "warn", module = "log")
    private static native void wasmImportWarn(int p0, int p1);
    
    public static void warn(String s) {
        byte[] bytes = (s).getBytes(StandardCharsets.UTF_8);
        wasmImportWarn(Address.ofData(bytes).toInt(), bytes.length);
        
    }
    @Import(name = "error", module = "log")
    private static native void wasmImportError(int p0, int p1);
    
    public static void error(String s) {
        byte[] bytes = (s).getBytes(StandardCharsets.UTF_8);
        wasmImportError(Address.ofData(bytes).toInt(), bytes.length);
        
    }
    
}

