package wit_trinity_module;

import java.nio.charset.StandardCharsets;
import java.util.ArrayList;

import org.teavm.interop.Memory;
import org.teavm.interop.Address;
import org.teavm.interop.Import;
import org.teavm.interop.Export;

public final class TrinityModule {
    private TrinityModule() {}
    
    public static final class Message {
        public final String content;
        public final String to;
        
        public Message(String content, String to) {
            this.content = content;
            this.to = to;
        }
    }
    
    @Export(name = "init")
    private static void wasmExportInit() {
        
        TrinityModuleImpl.init();
        
    }
    
    @Export(name = "help")
    private static int wasmExportHelp(int p0, int p1, int p2) {
        
        String lifted;
        
        switch (p0) {
            case 0: {
                lifted = null;
                break;
            }
            
            case 1: {
                
                byte[] bytes = new byte[p2];
                Memory.getBytes(Address.fromInt(p1), bytes, 0, p2);
                
                lifted = new String(bytes, StandardCharsets.UTF_8);
                break;
            }
            
            default: throw new AssertionError("invalid discriminant: " + (p0));
        }
        
        String result = TrinityModuleImpl.help(lifted);
        
        byte[] bytes2 = (result).getBytes(StandardCharsets.UTF_8);
        
        Address address = Memory.malloc(bytes2.length, 1);
        Memory.putBytes(address, bytes2, 0, bytes2.length);
        Address.fromInt((TrinityModuleWorld.RETURN_AREA) + 4).putInt(bytes2.length);
        Address.fromInt((TrinityModuleWorld.RETURN_AREA) + 0).putInt(address.toInt());
        return TrinityModuleWorld.RETURN_AREA;
        
    }
    
    @Export(name = "cabi_post_help")
    private static void wasmExportHelpPostReturn(int p0) {
        Memory.free(Address.fromInt(Address.fromInt((p0) + 0).getInt()), Address.fromInt((p0) + 4).getInt(), 1);
        
    }
    
    @Export(name = "admin")
    private static int wasmExportAdmin(int p0, int p1, int p2, int p3) {
        
        byte[] bytes = new byte[p1];
        Memory.getBytes(Address.fromInt(p0), bytes, 0, p1);
        
        byte[] bytes0 = new byte[p3];
        Memory.getBytes(Address.fromInt(p2), bytes0, 0, p3);
        
        ArrayList<Message> result = TrinityModuleImpl.admin(new String(bytes, StandardCharsets.UTF_8), new String(bytes0, StandardCharsets.UTF_8));
        
        int address4 = Memory.malloc((result).size() * 16, 4).toInt();
        for (int index = 0; index < (result).size(); ++index) {
            Message element = (result).get(index);
            int base = address4 + (index * 16);
            byte[] bytes1 = ((element).content).getBytes(StandardCharsets.UTF_8);
            
            Address address = Memory.malloc(bytes1.length, 1);
            Memory.putBytes(address, bytes1, 0, bytes1.length);
            Address.fromInt((base) + 4).putInt(bytes1.length);
            Address.fromInt((base) + 0).putInt(address.toInt());
            byte[] bytes2 = ((element).to).getBytes(StandardCharsets.UTF_8);
            
            Address address3 = Memory.malloc(bytes2.length, 1);
            Memory.putBytes(address3, bytes2, 0, bytes2.length);
            Address.fromInt((base) + 12).putInt(bytes2.length);
            Address.fromInt((base) + 8).putInt(address3.toInt());
            
        }
        Address.fromInt((TrinityModuleWorld.RETURN_AREA) + 4).putInt((result).size());
        Address.fromInt((TrinityModuleWorld.RETURN_AREA) + 0).putInt(address4);
        return TrinityModuleWorld.RETURN_AREA;
        
    }
    
    @Export(name = "cabi_post_admin")
    private static void wasmExportAdminPostReturn(int p0) {
        
        for (int index = 0; index < (Address.fromInt((p0) + 4).getInt()); ++index) {
            int base = (Address.fromInt((p0) + 0).getInt()) + (index * 16);
            Memory.free(Address.fromInt(Address.fromInt((base) + 0).getInt()), Address.fromInt((base) + 4).getInt(), 1);
            Memory.free(Address.fromInt(Address.fromInt((base) + 8).getInt()), Address.fromInt((base) + 12).getInt(), 1);
            
        }
        Memory.free(Address.fromInt(Address.fromInt((p0) + 0).getInt()), (Address.fromInt((p0) + 4).getInt()) * 16, 4);
        
    }
    
    @Export(name = "on-msg")
    private static int wasmExportOnMsg(int p0, int p1, int p2, int p3, int p4, int p5, int p6, int p7) {
        
        byte[] bytes = new byte[p1];
        Memory.getBytes(Address.fromInt(p0), bytes, 0, p1);
        
        byte[] bytes0 = new byte[p3];
        Memory.getBytes(Address.fromInt(p2), bytes0, 0, p3);
        
        byte[] bytes1 = new byte[p5];
        Memory.getBytes(Address.fromInt(p4), bytes1, 0, p5);
        
        byte[] bytes2 = new byte[p7];
        Memory.getBytes(Address.fromInt(p6), bytes2, 0, p7);
        
        ArrayList<Message> result = TrinityModuleImpl.onMsg(new String(bytes, StandardCharsets.UTF_8), new String(bytes0, StandardCharsets.UTF_8), new String(bytes1, StandardCharsets.UTF_8), new String(bytes2, StandardCharsets.UTF_8));
        
        int address6 = Memory.malloc((result).size() * 16, 4).toInt();
        for (int index = 0; index < (result).size(); ++index) {
            Message element = (result).get(index);
            int base = address6 + (index * 16);
            byte[] bytes3 = ((element).content).getBytes(StandardCharsets.UTF_8);
            
            Address address = Memory.malloc(bytes3.length, 1);
            Memory.putBytes(address, bytes3, 0, bytes3.length);
            Address.fromInt((base) + 4).putInt(bytes3.length);
            Address.fromInt((base) + 0).putInt(address.toInt());
            byte[] bytes4 = ((element).to).getBytes(StandardCharsets.UTF_8);
            
            Address address5 = Memory.malloc(bytes4.length, 1);
            Memory.putBytes(address5, bytes4, 0, bytes4.length);
            Address.fromInt((base) + 12).putInt(bytes4.length);
            Address.fromInt((base) + 8).putInt(address5.toInt());
            
        }
        Address.fromInt((TrinityModuleWorld.RETURN_AREA) + 4).putInt((result).size());
        Address.fromInt((TrinityModuleWorld.RETURN_AREA) + 0).putInt(address6);
        return TrinityModuleWorld.RETURN_AREA;
        
    }
    
    @Export(name = "cabi_post_on-msg")
    private static void wasmExportOnMsgPostReturn(int p0) {
        
        for (int index = 0; index < (Address.fromInt((p0) + 4).getInt()); ++index) {
            int base = (Address.fromInt((p0) + 0).getInt()) + (index * 16);
            Memory.free(Address.fromInt(Address.fromInt((base) + 0).getInt()), Address.fromInt((base) + 4).getInt(), 1);
            Memory.free(Address.fromInt(Address.fromInt((base) + 8).getInt()), Address.fromInt((base) + 12).getInt(), 1);
            
        }
        Memory.free(Address.fromInt(Address.fromInt((p0) + 0).getInt()), (Address.fromInt((p0) + 4).getInt()) * 16, 4);
        
    }
    
}

