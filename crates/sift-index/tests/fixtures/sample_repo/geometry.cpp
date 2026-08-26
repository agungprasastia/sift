namespace geo {

class Shape {
public:
    virtual double area() const { return 0.0; }
};

double total_area(const Shape& s) {
    return s.area();
}

}
