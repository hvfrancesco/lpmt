#include "testApp.h"

//--------------------------------------------------------------

void testApp::setXml()

{
    XML.setValue("GENERAL:ACTIVE_QUAD",activeQuad);
    XML.setValue("GENERAL:N_OF_QUADS",nOfQuads);
    XML.setValue("TIMELINE:USE_TIMELINE",useTimeline);
    XML.setValue("TIMELINE:DURATION",timelineDurationSeconds);
    for(int j=0; j<4; j++)
    {
        XML.setValue("SHARED_VIDEOS:VIDEO_"+ofToString(j)+":PATH", sharedVideosFiles[j]);
    }

    for(int i = 0; i < 36; i++)
    {
        if (quads[i].initialized)
        {

            XML.setValue("QUADS:QUAD_"+ofToString(i)+":NUMBER",quads[i].quadNumber);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":LAYER",quads[i].layer);

            XML.setValue("QUADS:QUAD_"+ofToString(i)+":TIMELINE:TINT",quads[i].bTimelineTint);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":TIMELINE:COLOR",quads[i].bTimelineColor);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":TIMELINE:ALPHA",quads[i].bTimelineAlpha);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":TIMELINE:SLIDE",quads[i].bTimelineSlideChange);

            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CONTENT:DISPX",quads[i].quadDispX);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CONTENT:DISPY",quads[i].quadDispY);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CONTENT:WIDTH",quads[i].quadW);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CONTENT:HEIGHT",quads[i].quadH);

            XML.setValue("QUADS:QUAD_"+ofToString(i)+":IMG:LOADED_IMG",quads[i].loadedImg);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":IMG:LOADED_IMG_PATH",quads[i].bgImg);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:LOADED_VIDEO",quads[i].loadedVideo);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:LOADED_VIDEO_PATH",quads[i].bgVideo);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":SLIDESHOW:LOADED_SLIDESHOW",quads[i].bgSlideshow);

            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_0:X",quads[i].corners[0].x);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_0:Y",quads[i].corners[0].y);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_1:X",quads[i].corners[1].x);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_1:Y",quads[i].corners[1].y);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_2:X",quads[i].corners[2].x);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_2:Y",quads[i].corners[2].y);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_3:X",quads[i].corners[3].x);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_3:Y",quads[i].corners[3].y);

            XML.setValue("QUADS:QUAD_"+ofToString(i)+":IS_ON",quads[i].isOn);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":COLOR:ACTIVE",quads[i].colorBg);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":COLOR:TRANS:ACTIVE",quads[i].transBg);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":COLOR:TRANS:DURATION",quads[i].transDuration);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":SLIDESHOW:ACTIVE",quads[i].slideshowBg);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":SLIDESHOW:SPEED",quads[i].slideshowSpeed);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":SLIDESHOW:FIT",quads[i].slideFit);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":SLIDESHOW:KEEP_ASPECT",quads[i].slideKeepAspect);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CAM:ACTIVE",quads[i].camBg);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":IMG:ACTIVE",quads[i].imgBg);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:ACTIVE",quads[i].videoBg);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CAM:WIDTH",quads[i].camWidth);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CAM:HEIGHT",quads[i].camHeight);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CAM:MULT_X",quads[i].camMultX);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CAM:MULT_Y",quads[i].camMultY);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":IMG:MULT_X",quads[i].imgMultX);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":IMG:MULT_Y",quads[i].imgMultY);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:MULT_X",quads[i].videoMultX);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:MULT_Y",quads[i].videoMultY);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:SPEED",quads[i].videoSpeed);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:VOLUME",quads[i].videoVolume);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:LOOP",quads[i].videoLoop);

            XML.setValue("QUADS:QUAD_"+ofToString(i)+":COLOR:R",quads[i].bgColor.r);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":COLOR:G",quads[i].bgColor.g);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":COLOR:B",quads[i].bgColor.b);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":COLOR:A",quads[i].bgColor.a);

            XML.setValue("QUADS:QUAD_"+ofToString(i)+":COLOR:SECOND_COLOR:R",quads[i].secondColor.r);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":COLOR:SECOND_COLOR:G",quads[i].secondColor.g);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":COLOR:SECOND_COLOR:B",quads[i].secondColor.b);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":COLOR:SECOND_COLOR:A",quads[i].secondColor.a);

            XML.setValue("QUADS:QUAD_"+ofToString(i)+":IMG:COLORIZE:R",quads[i].imgColorize.r);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":IMG:COLORIZE:G",quads[i].imgColorize.g);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":IMG:COLORIZE:B",quads[i].imgColorize.b);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":IMG:COLORIZE:A",quads[i].imgColorize.a);

            XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:COLORIZE:R",quads[i].videoColorize.r);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:COLORIZE:G",quads[i].videoColorize.g);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:COLORIZE:B",quads[i].videoColorize.b);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:COLORIZE:A",quads[i].videoColorize.a);

            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CAM:COLORIZE:R",quads[i].camColorize.r);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CAM:COLORIZE:G",quads[i].camColorize.g);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CAM:COLORIZE:B",quads[i].camColorize.b);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CAM:COLORIZE:A",quads[i].camColorize.a);

            XML.setValue("QUADS:QUAD_"+ofToString(i)+":IMG:FLIP:H",quads[i].imgHFlip);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":IMG:FLIP:V",quads[i].imgVFlip);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:FLIP:H",quads[i].videoHFlip);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:FLIP:V",quads[i].videoVFlip);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CAM:FLIP:H",quads[i].camHFlip);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CAM:FLIP:V",quads[i].camVFlip);

            XML.setValue("QUADS:QUAD_"+ofToString(i)+":SHARED_VIDEO:ACTIVE",quads[i].sharedVideoBg);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":SHARED_VIDEO:NUM",quads[i].sharedVideoNum);

            XML.setValue("QUADS:QUAD_"+ofToString(i)+":BLENDING:ON",quads[i].bBlendModes);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":BLENDING:MODE",quads[i].blendMode);

            XML.setValue("QUADS:QUAD_"+ofToString(i)+":EDGE_BLENDING:ON",quads[i].edgeBlendBool);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":EDGE_BLENDING:EXPONENT",quads[i].edgeBlendExponent);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":EDGE_BLENDING:GAMMA",quads[i].edgeBlendGamma);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":EDGE_BLENDING:LUMINANCE", quads[i].edgeBlendLuminance);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":EDGE_BLENDING:AMOUNT:SIN",quads[i].edgeBlendAmountSin);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":EDGE_BLENDING:AMOUNT:DX",quads[i].edgeBlendAmountDx);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":EDGE_BLENDING:AMOUNT:TOP",quads[i].edgeBlendAmountTop);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":EDGE_BLENDING:AMOUNT:BOTTOM",quads[i].edgeBlendAmountBottom);

            //mask stuff
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":MASK:ON",quads[i].bMask);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":MASK:INVERT_MASK",quads[i].maskInvert);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":MASK:N_POINTS",(int)quads[i].maskPoints.size());
            if (quads[i].maskPoints.size() > 0)
            {
                for(int j=0; j<quads[i].maskPoints.size(); j++)
                {
                    XML.setValue("QUADS:QUAD_"+ofToString(i)+":MASK:POINTS:POINT_"+ofToString(j)+":X",quads[i].maskPoints[j].x);
                    XML.setValue("QUADS:QUAD_"+ofToString(i)+":MASK:POINTS:POINT_"+ofToString(j)+":Y",quads[i].maskPoints[j].y);
                }
            }
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":MASK:CROP:RECTANGULAR:TOP",quads[i].crop[0]);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":MASK:CROP:RECTANGULAR:RIGHT",quads[i].crop[1]);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":MASK:CROP:RECTANGULAR:BOTTOM",quads[i].crop[2]);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":MASK:CROP:RECTANGULAR:LEFT",quads[i].crop[3]);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":MASK:CROP:CIRCULAR:X",quads[i].circularCrop[0]);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":MASK:CROP:CIRCULAR:Y",quads[i].circularCrop[1]);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":MASK:CROP:CIRCULAR:RADIUS",quads[i].circularCrop[2]);
            // deform stuff
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":DEFORM:ON",quads[i].bDeform);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":DEFORM:BEZIER:ON",quads[i].bBezier);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":DEFORM:GRID:ON",quads[i].bGrid);
            // bezier stuff
            for (int j = 0; j < 4; ++j)
            {
                for (int k = 0; k < 4; ++k)
                {
                    XML.setValue("QUADS:QUAD_"+ofToString(i)+":DEFORM:BEZIER:CTRLPOINTS:POINT_"+ofToString(j)+"_"+ofToString(k)+":X",quads[i].bezierPoints[j][k][0]);
                    XML.setValue("QUADS:QUAD_"+ofToString(i)+":DEFORM:BEZIER:CTRLPOINTS:POINT_"+ofToString(j)+"_"+ofToString(k)+":Y",quads[i].bezierPoints[j][k][1]);
                    XML.setValue("QUADS:QUAD_"+ofToString(i)+":DEFORM:BEZIER:CTRLPOINTS:POINT_"+ofToString(j)+"_"+ofToString(k)+":Z",quads[i].bezierPoints[j][k][2]);
                }
            }

        }
    }
}


void testApp::getXml(string xmlFile)

{
    XML.loadFile(xmlFile);
    cout<<"loaded settings from "+xmlFile<<endl;

    nOfQuads = XML.getValue("GENERAL:N_OF_QUADS", 0);
    activeQuad = XML.getValue("GENERAL:ACTIVE_QUAD", 0);
    useTimeline = XML.getValue("TIMELINE:USE_TIMELINE",0);
    timelineDurationSeconds = XML.getValue("TIMELINE:DURATION",100);

    for(int j=0; j<4; j++)
    {
        string sharedVideoPath = XML.getValue("SHARED_VIDEOS:VIDEO_"+ofToString(j)+":PATH", "");
        sharedVideosFiles[j] = sharedVideoPath;
        if(sharedVideoPath != "")
        {
            openSharedVideoFile(sharedVideoPath, j);
        }
    }

    for(int i = 0; i < nOfQuads; i++)
    {
        float x0 = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_0:X",0.0);
        float y0 = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_0:Y",0.0);
        float x1 = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_1:X",0.0);
        float y1 = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_1:Y",0.0);
        float x2 = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_2:X",0.0);
        float y2 = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_2:Y",0.0);
        float x3 = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_3:X",0.0);
        float y3 = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_3:Y",0.0);

        #ifdef WITH_KINECT
            #ifdef WITH_SYPHON
            quads[i].setup(x0, y0, x1, y1, x2, y2, x3, y3, edgeBlendShader, quadMaskShader, chromaShader, cameras, sharedVideos, kinect, syphClient);
            #else
            quads[i].setup(x0, y0, x1, y1, x2, y2, x3, y3, edgeBlendShader, quadMaskShader, chromaShader, cameras, sharedVideos, kinect);
            #endif
        #else
            #ifdef WITH_SYPHON
            quads[i].setup(x0, y0, x1, y1, x2, y2, x3, y3, edgeBlendShader, quadMaskShader, chromaShader, cameras, sharedVideos, syphClient);
            #else
            quads[i].setup(x0, y0, x1, y1, x2, y2, x3, y3, edgeBlendShader, quadMaskShader, chromaShader, cameras, sharedVideos);
            #endif
        #endif
        quads[i].quadNumber = XML.getValue("QUADS:QUAD_"+ofToString(i)+":NUMBER", 0);
        quads[i].layer = XML.getValue("QUADS:QUAD_"+ofToString(i)+":LAYER", 0);
        layers[quads[i].layer] = quads[i].quadNumber;

        quads[i].bTimelineTint = XML.getValue("QUADS:QUAD_"+ofToString(i)+":TIMELINE:TINT",0);
        quads[i].bTimelineColor = XML.getValue("QUADS:QUAD_"+ofToString(i)+":TIMELINE:COLOR",0);
        quads[i].bTimelineAlpha = XML.getValue("QUADS:QUAD_"+ofToString(i)+":TIMELINE:ALPHA",0);
        quads[i].bTimelineSlideChange = XML.getValue("QUADS:QUAD_"+ofToString(i)+":TIMELINE:SLIDE",0);

        quads[i].quadDispX = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CONTENT:DISPX",0);
        quads[i].quadDispY = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CONTENT:DISPY",0);
        quads[i].quadW = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CONTENT:WIDTH",0);
        quads[i].quadH = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CONTENT:HEIGHT",0);

        quads[i].imgBg = XML.getValue("QUADS:QUAD_"+ofToString(i)+":IMG:ACTIVE",0);
        quads[i].loadedImg = XML.getValue("QUADS:QUAD_"+ofToString(i)+":IMG:LOADED_IMG", "", 0);
        quads[i].bgImg = XML.getValue("QUADS:QUAD_"+ofToString(i)+":IMG:LOADED_IMG_PATH", "", 0);
        if ((quads[i].imgBg) && (quads[i].bgImg != ""))
        {
            quads[i].loadImageFromFile(quads[i].loadedImg, quads[i].bgImg);
        }
        quads[i].imgHFlip = XML.getValue("QUADS:QUAD_"+ofToString(i)+":IMG:FLIP:H", 0);
        quads[i].imgVFlip = XML.getValue("QUADS:QUAD_"+ofToString(i)+":IMG:FLIP:V", 0);

        quads[i].videoBg = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:ACTIVE",0);
        quads[i].loadedVideo = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:LOADED_VIDEO", "", 0);
        quads[i].bgVideo = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:LOADED_VIDEO_PATH", "", 0);
        if ((quads[i].videoBg) && (quads[i].bgVideo != ""))
        {
            quads[i].loadVideoFromFile(quads[i].loadedVideo, quads[i].bgVideo);
        }
        quads[i].videoHFlip = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:FLIP:H", 0);
        quads[i].videoVFlip = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:FLIP:V", 0);

        quads[i].sharedVideoBg = XML.getValue("QUADS:QUAD_"+ofToString(i)+":SHARED_VIDEO:ACTIVE",0);
        quads[i].sharedVideoNum = XML.getValue("QUADS:QUAD_"+ofToString(i)+":SHARED_VIDEO:NUM", 1);

        quads[i].bgSlideshow = XML.getValue("QUADS:QUAD_"+ofToString(i)+":SLIDESHOW:LOADED_SLIDESHOW", 0);

        quads[i].colorBg = XML.getValue("QUADS:QUAD_"+ofToString(i)+":COLOR:ACTIVE",0);

        quads[i].transBg = XML.getValue("QUADS:QUAD_"+ofToString(i)+":COLOR:TRANS:ACTIVE",0);
        quads[i].transDuration = XML.getValue("QUADS:QUAD_"+ofToString(i)+":COLOR:TRANS:DURATION", 1.0);
        quads[i].slideshowBg = XML.getValue("QUADS:QUAD_"+ofToString(i)+":SLIDESHOW:ACTIVE", 0);
        quads[i].slideshowSpeed = XML.getValue("QUADS:QUAD_"+ofToString(i)+":SLIDESHOW:SPEED", 1.0);
        quads[i].slideFit = XML.getValue("QUADS:QUAD_"+ofToString(i)+":SLIDESHOW:FIT", 0);
        quads[i].slideKeepAspect = XML.getValue("QUADS:QUAD_"+ofToString(i)+":SLIDESHOW:KEEP_ASPECT", 1);

        quads[i].camBg = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CAM:ACTIVE",0);
        quads[i].camWidth = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CAM:WIDTH",0);
        quads[i].camHeight = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CAM:HEIGHT",0);
        quads[i].camHFlip = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CAM:FLIP:H", 0);
        quads[i].camVFlip = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CAM:FLIP:V", 0);

        quads[i].camMultX = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CAM:MULT_X",1.0);
        quads[i].camMultY = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CAM:MULT_Y",1.0);
        quads[i].imgMultX = XML.getValue("QUADS:QUAD_"+ofToString(i)+":IMG:MULT_X",1.0);
        quads[i].imgMultY = XML.getValue("QUADS:QUAD_"+ofToString(i)+":IMG:MULT_Y",1.0);
        quads[i].videoMultX = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:MULT_X",1.0);
        quads[i].videoMultY = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:MULT_Y",1.0);
        quads[i].videoSpeed = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:SPEED",1.0);
        quads[i].videoVolume = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:VOLUME",0);
        quads[i].videoLoop = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:LOOP",1);

        quads[i].bgColor.r = XML.getValue("QUADS:QUAD_"+ofToString(i)+":COLOR:R",0.0);
        quads[i].bgColor.g = XML.getValue("QUADS:QUAD_"+ofToString(i)+":COLOR:G",0.0);
        quads[i].bgColor.b = XML.getValue("QUADS:QUAD_"+ofToString(i)+":COLOR:B",0.0);
        quads[i].bgColor.a = XML.getValue("QUADS:QUAD_"+ofToString(i)+":COLOR:A",0.0);

        quads[i].secondColor.r = XML.getValue("QUADS:QUAD_"+ofToString(i)+":COLOR:SECOND_COLOR:R",0.0);
        quads[i].secondColor.g = XML.getValue("QUADS:QUAD_"+ofToString(i)+":COLOR:SECOND_COLOR:G",0.0);
        quads[i].secondColor.b = XML.getValue("QUADS:QUAD_"+ofToString(i)+":COLOR:SECOND_COLOR:B",0.0);
        quads[i].secondColor.a = XML.getValue("QUADS:QUAD_"+ofToString(i)+":COLOR:SECOND_COLOR:A",0.0);

        quads[i].imgColorize.r = XML.getValue("QUADS:QUAD_"+ofToString(i)+":IMG:COLORIZE:R",1.0);
        quads[i].imgColorize.g = XML.getValue("QUADS:QUAD_"+ofToString(i)+":IMG:COLORIZE:G",1.0);
        quads[i].imgColorize.b = XML.getValue("QUADS:QUAD_"+ofToString(i)+":IMG:COLORIZE:B",1.0);
        quads[i].imgColorize.a = XML.getValue("QUADS:QUAD_"+ofToString(i)+":IMG:COLORIZE:A",1.0);

        quads[i].videoColorize.r = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:COLORIZE:R",1.0);
        quads[i].videoColorize.g = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:COLORIZE:G",1.0);
        quads[i].videoColorize.b = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:COLORIZE:B",1.0);
        quads[i].videoColorize.a = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:COLORIZE:A",1.0);

        quads[i].camColorize.r = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CAM:COLORIZE:R",1.0);
        quads[i].camColorize.g = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CAM:COLORIZE:G",1.0);
        quads[i].camColorize.b = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CAM:COLORIZE:B",1.0);
        quads[i].camColorize.a = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CAM:COLORIZE:A",1.0);

        quads[i].bBlendModes = XML.getValue("QUADS:QUAD_"+ofToString(i)+":BLENDING:ON", 0);
        quads[i].blendMode= XML.getValue("QUADS:QUAD_"+ofToString(i)+":BLENDING:MODE", 0);

        quads[i].edgeBlendBool = XML.getValue("QUADS:QUAD_"+ofToString(i)+":EDGE_BLENDING:ON", 0);
        quads[i].edgeBlendExponent = XML.getValue("QUADS:QUAD_"+ofToString(i)+":EDGE_BLENDING:EXPONENT", 1.0);
        quads[i].edgeBlendGamma = XML.getValue("QUADS:QUAD_"+ofToString(i)+":EDGE_BLENDING:GAMMA", 1.8);
        quads[i].edgeBlendLuminance = XML.getValue("QUADS:QUAD_"+ofToString(i)+":EDGE_BLENDING:LUMINANCE", 0.0);
        quads[i].edgeBlendAmountSin = XML.getValue("QUADS:QUAD_"+ofToString(i)+":EDGE_BLENDING:AMOUNT:SIN", 0.3);
        quads[i].edgeBlendAmountDx = XML.getValue("QUADS:QUAD_"+ofToString(i)+":EDGE_BLENDING:AMOUNT:DX", 0.3);
        quads[i].edgeBlendAmountTop = XML.getValue("QUADS:QUAD_"+ofToString(i)+":EDGE_BLENDING:AMOUNT:TOP", 0.0);
        quads[i].edgeBlendAmountBottom = XML.getValue("QUADS:QUAD_"+ofToString(i)+":EDGE_BLENDING:AMOUNT:BOTTOM", 0.0);

        //mask stuff
        quads[i].bMask = XML.getValue("QUADS:QUAD_"+ofToString(i)+":MASK:ON", 0);
        quads[i].maskInvert = XML.getValue("QUADS:QUAD_"+ofToString(i)+":MASK:INVERT_MASK", 0);
        int nOfMaskPoints =  XML.getValue("QUADS:QUAD_"+ofToString(i)+":MASK:N_POINTS", 0);
        quads[i].maskPoints.clear();
        if (nOfMaskPoints > 0)
        {
            for(int j=0; j<nOfMaskPoints; j++)
            {
                ofPoint tempMaskPoint;
                tempMaskPoint.x = XML.getValue("QUADS:QUAD_"+ofToString(i)+":MASK:POINTS:POINT_"+ofToString(j)+":X", 0);
                tempMaskPoint.y = XML.getValue("QUADS:QUAD_"+ofToString(i)+":MASK:POINTS:POINT_"+ofToString(j)+":Y", 0);
                quads[i].maskPoints.push_back(tempMaskPoint);
            }
        }
        quads[i].crop[0] = XML.getValue("QUADS:QUAD_"+ofToString(i)+":MASK:CROP:RECTANGULAR:TOP",0.0);
        quads[i].crop[1] = XML.getValue("QUADS:QUAD_"+ofToString(i)+":MASK:CROP:RECTANGULAR:RIGHT",0.0);
        quads[i].crop[2] = XML.getValue("QUADS:QUAD_"+ofToString(i)+":MASK:CROP:RECTANGULAR:BOTTOM",0.0);
        quads[i].crop[3] = XML.getValue("QUADS:QUAD_"+ofToString(i)+":MASK:CROP:RECTANGULAR:LEFT",0.0);
        quads[i].circularCrop[0] = XML.getValue("QUADS:QUAD_"+ofToString(i)+":MASK:CROP:CIRCULAR:X",0.5);
        quads[i].circularCrop[1] = XML.getValue("QUADS:QUAD_"+ofToString(i)+":MASK:CROP:CIRCULAR:Y",0.5);
        quads[i].circularCrop[2] = XML.getValue("QUADS:QUAD_"+ofToString(i)+":MASK:CROP:CIRCULAR:RADIUS",0.0);
        // deform stuff
        quads[i].bDeform = XML.getValue("QUADS:QUAD_"+ofToString(i)+":DEFORM:ON",0);
        quads[i].bBezier = XML.getValue("QUADS:QUAD_"+ofToString(i)+":DEFORM:BEZIER:ON",0);
        quads[i].bGrid = XML.getValue("QUADS:QUAD_"+ofToString(i)+":DEFORM:GRID:ON",0);
        // bezier stuff
        for (int j = 0; j < 4; ++j)
        {
            for (int k = 0; k < 4; ++k)
            {
                quads[i].bezierPoints[j][k][0] = XML.getValue("QUADS:QUAD_"+ofToString(i)+":DEFORM:BEZIER:CTRLPOINTS:POINT_"+ofToString(j)+"_"+ofToString(k)+":X",0.0);
                quads[i].bezierPoints[j][k][1] = XML.getValue("QUADS:QUAD_"+ofToString(i)+":DEFORM:BEZIER:CTRLPOINTS:POINT_"+ofToString(j)+"_"+ofToString(k)+":Y",0.0);
                quads[i].bezierPoints[j][k][2] = XML.getValue("QUADS:QUAD_"+ofToString(i)+":DEFORM:BEZIER:CTRLPOINTS:POINT_"+ofToString(j)+"_"+ofToString(k)+":Z",0.0);
            }
        }


        quads[i].isOn = XML.getValue("QUADS:QUAD_"+ofToString(i)+":IS_ON",0);
        quads[i].isActive = false;
    }
    quads[activeQuad].isActive = True;
    gui.setPage((activeQuad*3)+2);
    glDisable(GL_DEPTH_TEST);
}
