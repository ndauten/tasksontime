#!/usr/bin/ruby
#

class BrickTimeRecord
    attr_writer :startTime, :startTime, :name
    @startTime = 0
    @endTime = 0
    @name = ""
    def initialize(stime, etime, tags)
        @startTime = stime
        @endTime = etime
        @tags = Array.new
        @tags = tags
    end
    def to_s
        print "Name: #{@name} --- Start Time: #{@startTime} --- End Time: #{@endTime}"
    end
end

class Brick
    attr_reader :children, :parent, :name
    def initialize(name, parent, tags)
        @name = name
        @parent = parent
        @tags = Array.new           # This is an array of tags for this brick
        @tags.push(tags)
        @timeWorked = Array.new     # This is an array of TimeEntry objects
        @children = Hash.new       # Names of children -- index is a string
    end
    def addChild(child)
        @children[child.name] = child
    end
    def recordTime(te)
        @timeWorked.push(te)        
    end
    def to_s
        print "Brick Name: #{@name}, Parent: #{@parent}, Children: #{@children}, Tags: #{@tags}"
    end
end

class BrickTree
    def initialize()
        @root = Brick.new("root", "", ["all"])
        @tree = Hash.new
        @tree["root"] = {'brick' => @root, 'children' => {} }
    end

    def createBrick(parent, child, tags)
        newBrick = Brick.new(child,parent,tags)
        # Check that the parent node is in the tree
        raise 'Parent node does not exist.' unless @tree.has_key?(parent)
        @tree[parent]["brick"].addChild(child)
        @tree[parent]['children'] = { "brick" => newBrick, 'children' => {} }
        @tree[child] = newBrick
    end

    def addBrick(b)
        # Check that the parent node is in the tree
        raise 'Parent node does not exist.' unless @tree.has_key?(b.parent.name)
        @tree[b.parent.name]["brick"].addChild(b)
        @tree[b.parent.name]["children"][b] = b
        @tree[b.name] = {"brick" => b, "children" => b.children}
    end

    def isBrick(b)
        @tree.has_key?(b)
    end

    def hasChild(parent,child)
        # TODO: Need an exception here for parent does not exist 
        raise 'Parent node does not exist.' unless @tree.has_key?(parent)
        @tree[parent].children.include?(child)
    end
    
    def to_s
        @tree.each { |key,val|
            puts val
        }
        #root = @tree["root"]
        #root.children.each { |brickName|
            #puts brickName
            #brick = @tree[brickName]
            #print "\n Brick: #{brick}"
        #}
    end
end

def printTimeDiff(start)
    finish = Time.now()
    seconds = finish - start
    print "\n\t**** The time difference: "
    print "%02d:%02d:%02d ****\n" % [
        seconds / (60*60),
        seconds / 60 % 60,
        seconds % 60
    ]
    print "\nThe finish time: #{finish}\n"
end

#print "\tCommand [r]: "
#while(gets.chomp() =~ /rh/) do
#end
#puts "Starting..."
#begin
    #start = Time.now()
    #puts start
    #print "\tCommand [r]: "
    #while(gets.chomp() =~ /r/) do
        #printTimeDiff(start)
        #puts "\nRestarting..."
        #start = Time.now()
        #puts start
    #end
    #printTimeDiff(start)
#rescue Interrupt => e
    #print "\n"
    #printTimeDiff(start)
    #exit
#end
