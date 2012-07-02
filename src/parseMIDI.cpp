#include "testApp.h"

#ifdef WITH_MIDI

// MIDI message callback

//--------------------------------------------------------------
void testApp::newMidiMessage(ofxMidiMessage& msg) {

	// make a copy of the latest message
	midiMessage = msg;
	// following is for debug purposes
	/*
	cout << "status: " << midiMessage.status << endl;
	cout << "status string: " << midiMessage.getStatusString(midiMessage.status) << endl;
	cout << "channel: " << midiMessage.channel << endl;
	cout << "pitch: " << midiMessage.pitch << endl;
	cout << "velocity: " << midiMessage.velocity << endl;
	cout << "control: " << midiMessage.control << endl;
	cout << "value: " << midiMessage.value << endl;
	*/

	for(int i=0; i < gui.getPages().size(); i++)
	{
	    for(int j=0; j < gui.getPages()[i]->getControls().size(); j++)
	    {
	        if(gui.getPages()[i]->getControls()[j]->controlType == "Toggle")
	        {
                if(gui.getPages()[i]->getControls()[j]->bLearning)
                {
                gui.getPages()[i]->getControls()[j]->bLearning = false;
                gui.getPages()[i]->getControls()[j]->bLearnt = true;
                gui.getPages()[i]->getControls()[j]->midiControl = midiMessage;
                }
                else if(gui.getPages()[i]->getControls()[j]->bLearnt)
                {
                    ofxMidiMessage midiControl = gui.getPages()[i]->getControls()[j]->midiControl;
                    if(midiMessage.status == midiControl.status && midiMessage.pitch == midiControl.pitch && midiMessage.velocity == midiControl.velocity && midiMessage.channel == midiControl.channel)
                    {
                    ofxSimpleGuiToggle *t = (ofxSimpleGuiToggle *) gui.getPages()[i]->getControls()[j];
                    t->toggle();
                    }
                }
	        }
	        else if(gui.getPages()[i]->getControls()[j]->controlType == "SliderFloat" || gui.getPages()[i]->getControls()[j]->controlType == "SliderInt")
	        {
	            if(gui.getPages()[i]->getControls()[j]->bLearning)
                {
                    if(midiMessage.status == MIDI_CONTROL_CHANGE || midiMessage.status == MIDI_PITCH_BEND)
                    {
                        gui.getPages()[i]->getControls()[j]->bLearning = false;
                        gui.getPages()[i]->getControls()[j]->bLearnt = true;
                        gui.getPages()[i]->getControls()[j]->midiControl = midiMessage;
                    }
                }
                else if(gui.getPages()[i]->getControls()[j]->bLearnt)
                {
                    ofxMidiMessage midiControl = gui.getPages()[i]->getControls()[j]->midiControl;
                    if(midiControl.status == MIDI_CONTROL_CHANGE)
                    {
                        if(midiMessage.status == midiControl.status && midiMessage.control == midiControl.control && midiMessage.channel == midiControl.channel)
                        {
                            if(gui.getPages()[i]->getControls()[j]->controlType == "SliderFloat")
                            {
                                ofxSimpleGuiSliderFloat *s = (ofxSimpleGuiSliderFloat *) gui.getPages()[i]->getControls()[j];
                                float value = midiMessage.value;
                                float remappedValue = ofMap(value, (float) 0, (float) 127, (float) s->min, (float) s->max);
                                s->setValue(remappedValue);
                            }
                            else
                            {
                                ofxSimpleGuiSliderInt *s = (ofxSimpleGuiSliderInt *) gui.getPages()[i]->getControls()[j];
                                float value = midiMessage.value;
                                float remappedValue = ofMap(value, (float) 0, (float) 127, (float) s->min, (float) s->max);
                                s->setValue((int)remappedValue);
                            }
                        }
                    }
                    else if(midiControl.status == MIDI_PITCH_BEND)
                    {
                        if(midiMessage.status == midiControl.status && midiMessage.channel == midiControl.channel)
                        {
                            if(gui.getPages()[i]->getControls()[j]->controlType == "SliderFloat")
                            {
                                ofxSimpleGuiSliderFloat *s = (ofxSimpleGuiSliderFloat *) gui.getPages()[i]->getControls()[j];
                                float value = midiMessage.value;
                                float remappedValue = ofMap(value, (float) 0, (float) 16383, (float) s->min, (float) s->max);
                                s->setValue(remappedValue);
                            }
                            else
                            {
                                ofxSimpleGuiSliderInt *s = (ofxSimpleGuiSliderInt *) gui.getPages()[i]->getControls()[j];
                                float value = midiMessage.value;
                                float remappedValue = ofMap(value, (float) 0, (float) 16383, (float) s->min, (float) s->max);
                                s->setValue((int)remappedValue);
                            }
                        }
                    }
                }
	        }
	    }
	}
}

#endif
