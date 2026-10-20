#include "quad.h"

// Bezier helpers --------------------------------------
// Bezier setup -------------------------------------
void quad::bezierSurfaceSetup()
{
    float tmp_bezierPoints[4][4][3] =
    {
        {   {0, 0, 0},          {0.333, 0, 0},    {0.667, 0, 0},    {1.0, 0, 0}    },
        {   {0, 0.333, 0},        {0.333, 0.333, 0},  {0.667, 0.333, 0},  {1.0, 0.333, 0}  },
        {   {0, 0.667, 0},        {0.333, 0.667, 0},  {0.667, 0.667, 0},  {1.0, 0.667, 0}  },
        {   {0, 1.0, 0},        {0.333, 1.0, 0},  {0.667, 1.0, 0},  {1.0, 1.0, 0}  }
    };

	for (int i = 0; i < 4; ++i) {
		for (int j = 0; j < 4; ++j) {
			for (int k = 0; k < 3; ++k) {
				bezierPoints[i][j][k] = tmp_bezierPoints[i][j][k];
			}
		}
	}

	//This sets up my Bezier Surface
    float tmp_ctrlPoints[4][4][3] = {
        {   {bezierPoints[0][0][0]*ofGetWidth(), bezierPoints[0][0][1]*ofGetHeight(), 0}, {bezierPoints[0][1][0]*ofGetWidth(), bezierPoints[0][1][1]*ofGetHeight(), 0}, {bezierPoints[0][2][0]*ofGetWidth(), bezierPoints[0][2][1]*ofGetHeight(), 0}, {bezierPoints[0][3][0]*ofGetWidth(), bezierPoints[0][3][1]*ofGetHeight(), 0} },
        {   {bezierPoints[1][0][0]*ofGetWidth(), bezierPoints[1][0][1]*ofGetHeight(), 0}, {bezierPoints[1][1][0]*ofGetWidth(), bezierPoints[1][1][1]*ofGetHeight(), 0}, {bezierPoints[1][2][0]*ofGetWidth(), bezierPoints[1][2][1]*ofGetHeight(), 0}, {bezierPoints[1][3][0]*ofGetWidth(), bezierPoints[1][3][1]*ofGetHeight(), 0}  },
        {   {bezierPoints[2][0][0]*ofGetWidth(), bezierPoints[2][0][1]*ofGetHeight(), 0}, {bezierPoints[2][1][0]*ofGetWidth(), bezierPoints[2][1][1]*ofGetHeight(), 0}, {bezierPoints[2][2][0]*ofGetWidth(), bezierPoints[2][2][1]*ofGetHeight(), 0}, {bezierPoints[2][3][0]*ofGetWidth(), bezierPoints[2][3][1]*ofGetHeight(), 0}  },
        {   {bezierPoints[3][0][0]*ofGetWidth(), bezierPoints[3][0][1]*ofGetHeight(), 0}, {bezierPoints[3][1][0]*ofGetWidth(), bezierPoints[3][1][1]*ofGetHeight(), 0}, {bezierPoints[3][2][0]*ofGetWidth(), bezierPoints[3][2][1]*ofGetHeight(), 0}, {bezierPoints[3][3][0]*ofGetWidth(), bezierPoints[3][3][1]*ofGetHeight(), 0}  }
    };

	for (int i = 0; i < 4; ++i) {
		for (int j = 0; j < 4; ++j) {
			for (int k = 0; k < 3; ++k) {
				bezierCtrlPoints[i][j][k] = tmp_ctrlPoints[i][j][k];
			}
		}
	}

    //This sets up my Texture Surface
    GLfloat texpts [2][2][2] =
    {
        { {0, 0}, {1, 0} }, { {0, 1}, {1, 1} }
    };

    // enable depth test, so we only see the front
    glEnable(GL_DEPTH_TEST);
    //set up bezier surface with a 4th order evaluator
    glMap2f(GL_MAP2_VERTEX_3, 0, 1, 3, 4, 0, 1, 12, 4, &bezierCtrlPoints[0][0][0]);
    //set up texture map for bezier surface
    glMap2f(GL_MAP2_TEXTURE_COORD_2, 0, 1, 2, 2, 0, 1, 4, 2, &texpts[0][0][0]);
    glEnable(GL_MAP2_TEXTURE_COORD_2);
    glEnable(GL_MAP2_VERTEX_3);
    glEnable(GL_AUTO_NORMAL);
    glMapGrid2f(20, 0, 1, 20, 0, 1);
    glShadeModel(GL_FLAT);
}

void quad::bezierSurfaceUpdate()
{
    // TODO: to optimize this try to limit recalculation to cases when it's really needed
   float tmp_ctrlPoints[4][4][3]  =
        {
            {   {bezierPoints[0][0][0]*ofGetWidth(), bezierPoints[0][0][1]*ofGetHeight(), 0}, {bezierPoints[0][1][0]*ofGetWidth(), bezierPoints[0][1][1]*ofGetHeight(), 0}, {bezierPoints[0][2][0]*ofGetWidth(), bezierPoints[0][2][1]*ofGetHeight(), 0}, {bezierPoints[0][3][0]*ofGetWidth(), bezierPoints[0][3][1]*ofGetHeight(), 0} },
            {   {bezierPoints[1][0][0]*ofGetWidth(), bezierPoints[1][0][1]*ofGetHeight(), 0}, {bezierPoints[1][1][0]*ofGetWidth(), bezierPoints[1][1][1]*ofGetHeight(), 0}, {bezierPoints[1][2][0]*ofGetWidth(), bezierPoints[1][2][1]*ofGetHeight(), 0}, {bezierPoints[1][3][0]*ofGetWidth(), bezierPoints[1][3][1]*ofGetHeight(), 0}  },
            {   {bezierPoints[2][0][0]*ofGetWidth(), bezierPoints[2][0][1]*ofGetHeight(), 0}, {bezierPoints[2][1][0]*ofGetWidth(), bezierPoints[2][1][1]*ofGetHeight(), 0}, {bezierPoints[2][2][0]*ofGetWidth(), bezierPoints[2][2][1]*ofGetHeight(), 0}, {bezierPoints[2][3][0]*ofGetWidth(), bezierPoints[2][3][1]*ofGetHeight(), 0}  },
            {   {bezierPoints[3][0][0]*ofGetWidth(), bezierPoints[3][0][1]*ofGetHeight(), 0}, {bezierPoints[3][1][0]*ofGetWidth(), bezierPoints[3][1][1]*ofGetHeight(), 0}, {bezierPoints[3][2][0]*ofGetWidth(), bezierPoints[3][2][1]*ofGetHeight(), 0}, {bezierPoints[3][3][0]*ofGetWidth(), bezierPoints[3][3][1]*ofGetHeight(), 0}  }
        };

		for (int i = 0; i < 4; ++i) {
			for (int j = 0; j < 4; ++j) {
				for (int k = 0; k < 3; ++k) {
					bezierCtrlPoints[i][j][k] = tmp_ctrlPoints[i][j][k];
				}
			}
		}

    if(bBezier)
    {
        glMap2f(GL_MAP2_VERTEX_3, 0, 1, 3, 4, 0, 1, 12, 4, &bezierCtrlPoints[0][0][0]);
        GLfloat texpts [2][2][2] =
        {
            { {0, 0}, {1, 0} }, { {0, 1}, {1, 1} }
        };
        glMap2f(GL_MAP2_TEXTURE_COORD_2, 0, 1, 2, 2, 0, 1, 4, 2, &texpts[0][0][0]);
        glEnable(GL_MAP2_TEXTURE_COORD_2);
        glEnable(GL_MAP2_VERTEX_3);
        glEnable(GL_AUTO_NORMAL);
        glMapGrid2f(20, 0, 1, 20, 0, 1);
        glShadeModel(GL_FLAT);
    }

}



// Grid helpers --------------------------------------
// grid setup -------------------------------------
void quad::gridSurfaceSetup()
{

    gridMesh.clearVertices();
    gridMesh.clearIndices();
    gridMesh.clearTexCoords();

    for(int i=0; i<=gridRows; i++)
    {
        vector<vector<float> > row;
        for(int j=0; j<=gridColumns; j++)
        {
            vector<float> column;
            column.push_back((float)(1.0/gridColumns*j));
            column.push_back((float)(1.0/gridRows*i));
            column.push_back(0.0);
            row.push_back(column);
        }
        gridPoints.push_back(row);
    }


    for(int i=0; i<=gridRows; i++)
    {
        for(int j=0; j<=gridColumns; j++)
        {
            gridMesh.addTexCoord(ofVec2f((gridPoints[i][j][0]*ofGetWidth()), (gridPoints[i][j][1]*ofGetHeight())));
        }
    }


    for(int i=0; i<=gridRows; i++)
    {
        for(int j=0; j<=gridColumns; j++)
        {
            gridMesh.addVertex(ofVec3f((gridPoints[i][j][0]*ofGetWidth()), (gridPoints[i][j][1]*ofGetHeight()), (0.0)));
        }
    }

    for(int i=0; i<gridRows; i++)
    {
        for(int j=0; j<gridColumns; j++)
        {

            gridMesh.addIndex((i*(gridColumns+1)+j)); //a
            gridMesh.addIndex((i*(gridColumns+1)+j+1)); //b
            gridMesh.addIndex(((i+1)*(gridColumns+1)+j+1));  //c
            gridMesh.addIndex(((i+1)*(gridColumns+1)+j+1)); //c
            gridMesh.addIndex(((i+1)*(gridColumns+1)+j)); //d
            gridMesh.addIndex((i*(gridColumns+1)+j)); //a

        }
    }
}


void quad::gridSurfaceUpdate()
{
    // TODO: to optimize this try to limit recalculation to cases when it's really needed
    //This sets up my Grid Surface
    gridMesh.clearVertices();
    gridMesh.clearIndices();

    if(gridPoints.size() != (gridRows+1) || gridPoints[0].size() != (gridColumns+1))
    {
        gridMesh.clearTexCoords();
        gridPoints.clear();
        for(int i=0; i<=gridRows; i++)
        {
            vector<vector<float> > row;
            for(int j=0; j<=gridColumns; j++)
            {
                vector<float> column;
                column.push_back((float)(1.0/gridColumns*j));
                column.push_back((float)(1.0/gridRows*i));
                column.push_back(0.0);
                row.push_back(column);
            }
            gridPoints.push_back(row);
        }
        for(int i=0; i<=gridRows; i++)
        {
            for(int j=0; j<=gridColumns; j++)
            {
                gridMesh.addTexCoord(ofVec2f((gridPoints[i][j][0]*ofGetWidth()), (gridPoints[i][j][1]*ofGetHeight())));
            }
        }

    }

    for(int i=0; i<=gridRows; i++)
    {
        for(int j=0; j<=gridColumns; j++)
        {
            gridMesh.addVertex(ofVec3f((gridPoints[i][j][0]*ofGetWidth()), (gridPoints[i][j][1]*ofGetHeight()), (0.0)));
        }
    }

    for(int i=0; i<gridRows; i++)
    {
        for(int j=0; j<gridColumns; j++)
        {

            gridMesh.addIndex((i*(gridColumns+1)+j)); //a
            gridMesh.addIndex((i*(gridColumns+1)+j+1)); //b
            gridMesh.addIndex(((i+1)*(gridColumns+1)+j+1));  //c
            gridMesh.addIndex(((i+1)*(gridColumns+1)+j+1)); //c
            gridMesh.addIndex(((i+1)*(gridColumns+1)+j)); //d
            gridMesh.addIndex((i*(gridColumns+1)+j)); //a

        }
    }

    //glMap2f(GL_MAP2_VERTEX_3, 0, 1, 3, (gridColumns+1), 0, 1, (gridColumns+1)*3, (gridRows+1), &gridCtrlPoints[0]);

}


