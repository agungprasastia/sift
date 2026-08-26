#include <stdlib.h>

struct Point {
    int x;
    int y;
};

int point_area(struct Point p) {
    return p.x * p.y;
}

#define MAX_POINTS 128
