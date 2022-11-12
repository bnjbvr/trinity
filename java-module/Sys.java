package wit_trinity_module;

import java.nio.charset.StandardCharsets;
import java.util.ArrayList;

import org.teavm.interop.Memory;
import org.teavm.interop.Address;
import org.teavm.interop.Import;
import org.teavm.interop.Export;

public final class Sys {
    private Sys() {}
    
    @Import(name = "rand-u64", module = "sys")
    private static native long wasmImportRandU64();
    
    public static long randU64() {
        long result =  wasmImportRandU64();
        return result;
        
    }
    
}

