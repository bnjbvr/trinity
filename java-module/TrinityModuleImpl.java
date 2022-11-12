package wit_trinity_module;

import java.util.ArrayList;

public class TrinityModuleImpl {
    public static void init() {
        Log.trace("Hello, world!");
    }

    public static String help(String topic) {
        return "Requested help for topic " + topic;
    }

    public static ArrayList<TrinityModule.Message> admin(String cmd, String authorId) {
        return new ArrayList();
    }

    public static ArrayList<TrinityModule.Message> onMsg(String content, String authorId, String authorName, String room) {
        TrinityModule.Message msg = new TrinityModule.Message("Hello, " + authorId + "!", authorId);
        ArrayList<TrinityModule.Message> list = new ArrayList();
        list.push(msg);
        return list;
    }
}
