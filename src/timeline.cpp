#include "testApp.h"

//--------------------------------------------------------------
void testApp::timelineSetup(float duration){

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
	//timeline.setDurationInFrames(duration);
	timeline.setDurationInSeconds(duration);
    ofAddListener(ofxTLEvents.trigger, this, &testApp::timelineTriggerReceived);
    timeline.enableSnapToBPM(60.0);
    timeline.collapseAllElements();
}

//--------------------------------------------------------------
void testApp::timelineUpdate()
{
            for(int j = 0; j < 36; j++)
            {
                if (quads[j].initialized)
                {
                    if(quads[j].bTimelineColor)
                    {
                        quads[j].timelineRed = timeline.getKeyframeValue("red_"+ofToString(j));
                        quads[j].timelineGreen = timeline.getKeyframeValue("green_"+ofToString(j));
                        quads[j].timelineBlu = timeline.getKeyframeValue("blu_"+ofToString(j));
                    }
                    if(quads[j].bTimelineAlpha)
                    {
                        quads[j].timelineAlpha = timeline.getKeyframeValue("alpha_"+ofToString(j));
                    }
                }
            }
}

//--------------------------------------------------------------
void testApp::timelineTriggerReceived(ofxTLTriggerEventArgs& trigger){
    vector<string> triggerParts = ofSplitString(trigger.triggerGroupName, "_", true, true);

    if(useTimeline)
    {
	//cout << "Trigger from " << trigger.triggerGroupName << " says color " << trigger.triggerName << endl;
	//cout << "Trigger from " << ofToInt(triggerParts[1]) << " says " << trigger.triggerName << endl;

	    if (trigger.triggerName == "on"){ quads[ofToInt(triggerParts[1])].isOn=true; }
        else if (trigger.triggerName == "off"){ quads[ofToInt(triggerParts[1])].isOn=false; }
	    else if(trigger.triggerName == "img_on"){ quads[ofToInt(triggerParts[1])].imgBg=true; }
	    else if (trigger.triggerName == "img_off"){ quads[ofToInt(triggerParts[1])].imgBg=false; }
        else if (trigger.triggerName == "col_on"){ quads[ofToInt(triggerParts[1])].colorBg=true; }
        else if (trigger.triggerName == "col_off"){ quads[ofToInt(triggerParts[1])].colorBg=false; }
        else if (trigger.triggerName == "video_on"){ quads[ofToInt(triggerParts[1])].videoBg=true; }
        else if (trigger.triggerName == "video_off"){ quads[ofToInt(triggerParts[1])].videoBg=false; }
        else if (trigger.triggerName == "cam_on"){ quads[ofToInt(triggerParts[1])].camBg=true; }
        else if (trigger.triggerName == "cam_off"){ quads[ofToInt(triggerParts[1])].camBg=false; }
        else if (trigger.triggerName == "mask_on"){ quads[ofToInt(triggerParts[1])].bMask=true; }
        else if (trigger.triggerName == "mask_off"){ quads[ofToInt(triggerParts[1])].bMask=false; }
        else if (trigger.triggerName == "mask_invert_on"){ quads[ofToInt(triggerParts[1])].maskInvert=true; }
        else if (trigger.triggerName == "mask_invert_off"){ quads[ofToInt(triggerParts[1])].maskInvert=false; }
    }
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
