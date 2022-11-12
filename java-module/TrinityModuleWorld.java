package wit_trinity_module;

import java.nio.charset.StandardCharsets;
import java.util.ArrayList;

import org.teavm.interop.Memory;
import org.teavm.interop.Address;
import org.teavm.interop.Import;
import org.teavm.interop.Export;
import org.teavm.interop.CustomSection;

public final class TrinityModuleWorld {
    private TrinityModuleWorld() {}
    
    @CustomSection(name = "component-type:TrinityModule")
    private static final String __WIT_BINDGEN_COMPONENT_TYPE = "01000061736d0a00010007b2010b400001006b73400105746f706963010073720207636f6e74656e747302746f737003400203636d647309617574686f722d6964730004400407636f6e74656e747309617574686f722d6964730b617574686f722d6e616d657304726f6f6d7300044000007742020203020107040872616e642d753634010040010173730100420602030201090405747261636501000405646562756701000404696e666f010004047761726e010004056572726f7201000a0d02037379730508036c6f67050a0b2a05076d657373616765030304696e697403000468656c7003020561646d696e0305066f6e2d6d73670306";
    
    public static final int RETURN_AREA = Memory.malloc(8, 4).toInt();
}
