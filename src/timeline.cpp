#include "testApp.h"

//--------------------------------------------------------------
void testApp::timelineSetup(int duration){

    timeline.setup();

	timeline.setPageName("0"); //changes the first page name
	timeline.addKeyframes("red_0", "0_red.xml", ofRange(0, 1.0));
	timeline.addKeyframes("green_0", "0_green.xml", ofRange(0, 1.0));
	timeline.addKeyframes("blu_0", "0_blu.xml", ofRange(0, 1.0));
	timeline.addKeyframes("alpha_0", "0_alpha.xml", ofRange(0, 1.0));
	timeline.addTriggers("trigger_0", "0_trigger.xml");


    for(int i = 1; i < 4; i++)
    {
        timelineAddQuadPage(i);
    }

    timeline.setLoopType(OF_LOOP_NORMAL);
	timeline.setDurationInFrames(duration);
    ofAddListener(ofxTLEvents.trigger, this, &testApp::timelineTriggerReceived);
}

//--------------------------------------------------------------
void testApp::timelineTriggerReceived(ofxTLTriggerEventArgs& trigger){
    vector<string> triggerParts = ofSplitString(trigger.triggerGroupName, "_", true, true);
	//cout << "Trigger from " << trigger.triggerGroupName << " says color " << trigger.triggerName << endl;
	cout << "Trigger from " << triggerParts[1] << " says " << trigger.triggerName << endl;
	//switch (trigger.triggerName)
	//{
	//    case("img_on"):

	//}
}

//--------------------------------------------------------------
void testApp::timelineAddQuadPage(int i) {
    timeline.addPage(ofToString(i), true);
    timeline.addKeyframes("red_"+ofToString(i), ofToString(i)+"_red.xml", ofRange(0, 1.0));
	timeline.addKeyframes("green_"+ofToString(i), ofToString(i)+"_green.xml", ofRange(0, 1.0));
	timeline.addKeyframes("blu_"+ofToString(i), ofToString(i)+"_blu.xml", ofRange(0, 1.0));
	timeline.addKeyframes("alpha_"+ofToString(i), ofToString(i)+"_alpha.xml", ofRange(0, 1.0));
	timeline.addTriggers("trigger_"+ofToString(i), ofToString(i)+"_trigger.xml");
}
