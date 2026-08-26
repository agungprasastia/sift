package main

import "fmt"

func main() {
	fmt.Println("hi")
}

func helper(n int) int {
	return n * 2
}

type Server struct {
	Port int
}

type Handler interface {
	Handle()
}
