#include "testApp.h"
#include "stdio.h"
#include <iostream>


//--------------------------------------------------------------
void testApp::setup(){
	//we run at 60 fps!
	ofSetVerticalSync(true);



    	ttf.loadFont("type/frabk.ttf", 11);
	// set border color for quads in setup mode
	borderColor = 0xFF6600;
	// starts in quads setup mode
	isSetup = True;
	// defines the first 4 default quads
	quads[0].setup(0.0,0.0,0.5,0.0,0.5,0.5,0.0,0.5);
    	quads[1].setup(0.5,0.0,1.0,0.0,1.0,0.5,0.5,0.5);
    	quads[2].setup(0.0,0.5,0.5,0.5,0.5,1.0,0.0,1.0);
    	quads[3].setup(0.5,0.5,1.0,0.5,1.0,1.0,0.5,1.0);
	// define last one as active quad
	activeQuad = 3;
	// number of total quads, to be modified later at each quad insertion
	nOfQuads = 4;
}

//--------------------------------------------------------------
void testApp::update(){
	
	// sets default window background and fixed shape
	ofBackground(20, 20, 20);
	ofSetWindowShape(800, 600);
	// loops through initialized quads and runs update, setting the border color as well
    	for(int i = 0; i < 16; i++){
    		if (quads[i].initialized) {
		quads[i].update();
		quads[i].borderColor = borderColor;		
		}
	}
}

//--------------------------------------------------------------
void testApp::draw(){
	
	// in setup mode sets active quad border to be white
	if (isSetup) {
	quads[activeQuad].borderColor = 0xFFFFFF;
	}
	
	// loops through initialized quads and calls their draw function
	for(int i = 0; i < 16; i++){
	if (quads[i].initialized) {
		quads[i].draw();
		}
	}
	
	// in setup mode writes the number of active quad at the bottom of the window
	if (isSetup) {
	ofSetColor(0xFFFFFF);
	ttf.drawString("active quad: "+ofToString(activeQuad), 30, 570);
	}

}

//--------------------------------------------------------------
void testApp::keyPressed(int key){

if ( key =='r' || key == 'R' ) 
	{
	if (isSetup) 
	{
	quads[activeQuad].corners[0].x = 0.0;
	quads[activeQuad].corners[0].y = 0.0;

	quads[activeQuad].corners[1].x = 1.0;
	quads[activeQuad].corners[1].y = 0.0;

	quads[activeQuad].corners[2].x = 1.0;
	quads[activeQuad].corners[2].y = 1.0;

	quads[activeQuad].corners[3].x = 0.0;
	quads[activeQuad].corners[3].y = 1.0;
	}
	}

if ( key =='>' )
	{
	if (isSetup) {
	activeQuad += 1;
	if (activeQuad > nOfQuads-1) {activeQuad = 0;}
	}
	}	

if ( key =='<' )
	{
	if (isSetup) {
	activeQuad -= 1;
	if (activeQuad < 0) {activeQuad = nOfQuads-1;}
	}
	}

if ( key =='a' ) {
	if (isSetup) {
    	if (nOfQuads < 15) {
		quad q;
		quads[nOfQuads].setup(0.25,0.25,0.75,0.25,0.75,0.75,0.25,0.75);
		activeQuad = nOfQuads;
		++nOfQuads;
	}
	}
	}

if ( key ==' ' ) {
	if (isSetup) {
		isSetup = False;
		for(int i = 0; i < 16; i++){
			if (quads[i].initialized) { quads[i].isSetup = False; }
			}
	}
	else {
		isSetup = True;
		for(int i = 0; i < 16; i++){
			if (quads[i].initialized) { quads[i].isSetup = True;}
			}	
	}
	}

}

//--------------------------------------------------------------
void testApp::keyReleased(int key){
}


//--------------------------------------------------------------
void testApp::mouseMoved(int x, int y ){


}

//--------------------------------------------------------------
void testApp::mouseDragged(int x, int y, int button){
		if (isSetup) {

		float scaleX = (float)x / ofGetWidth();
		float scaleY = (float)y / ofGetHeight();

		if(whichCorner >= 0){
			quads[activeQuad].corners[whichCorner].x = scaleX;
			quads[activeQuad].corners[whichCorner].y = scaleY;
		}

}
}

//--------------------------------------------------------------
void testApp::mousePressed(int x, int y, int button){
	if (isSetup) {
	float smallestDist = 1.0;
	whichCorner = -1;

	for(int i = 0; i < 4; i++){
		float distx = quads[activeQuad].corners[i].x - (float)x/ofGetWidth();
		float disty = quads[activeQuad].corners[i].y - (float)y/ofGetHeight();
		float dist  = sqrt( distx * distx + disty * disty);

		if(dist < smallestDist && dist < 0.1){
			whichCorner = i;
			smallestDist = dist;
		}
	}
}
}

//--------------------------------------------------------------
void testApp::mouseReleased(){
	whichCorner = -1;
}


