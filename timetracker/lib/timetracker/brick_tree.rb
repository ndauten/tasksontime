#!/usr/bin/ruby
#

class BrickTimeRecord
    attr_reader :startTime, :endTime, :name
    attr_writer :startTime, :endTime, :name
    @startTime = 0
    @endTime = 0
    @name = ""

    def initialize(tname="", stime=0, etime=0, tags=[])
        @startTime = stime
        @endTime = etime
        @tags = Array.new
        @tags = tags
        @name = tname
    end

    def duration  #Note this is in seconds
        @endTime - @startTime
    end

    def onDate(date)
        date.day == @startTime.day &&
        date.month == @startTime.month &&
        date.year == @startTime.year
    end
    
    def inRange(beginDate, endDate)
        beginDate < @startTime && @startTime < endDate
    end

    def to_s
        stP = @startTime.strftime("[%m.%d.%y] %I:%M%p")
        etP = @endTime.strftime("%I:%M%p")
        "#{stP} - #{etP} [#{@name}]"
    end
end

class Brick
    attr_reader :children, :parent, :name
    def initialize(name, parent, budget=0, tags)
        @name = name
        @parent = parent
        @tags = Array.new           # This is an array of tags for this brick
        @tags.push(tags)
        @timeWorked = Array.new     # This is an array of TimeEntry objects
        @children = Hash.new       # Names of children -- index is a string
        @budget = 0
    end
    def addChild(child)
        @children[child.name] = child
    end
    def recordTime(te)
        @timeWorked.push(te)        
    end
    def to_s
        "Brick Name: #{@name}, Parent: #{@parent}, Children: #{@children}, Tags: #{@tags}"
    end
end

#
# ----------------------------------------------------------------------------
# Description: 
#   The BrickTree represents a tree of roles/categories/tasks/projects/bricks
#   where each node is something that time can be assigned to. In this way the
#   brick can have encoded by the tree all time associated with it by
#   accumulating the time worked in each of its children nodes. 
# ----------------------------------------------------------------------------
#
class BrickTree
    @@pomo_duration = 25 # TODO MAKE CONFIGURABLE

    def initialize()
        @tree = Hash.new
        @tree["root"] = {
            'brick' => "root", 
            'parent' => '', 
            'tags' => ["all"], 
            'timeWorked' => [], 
            'children' => [] 
        }
    end

    def addBrick(bname, parentName, budget, tags)
        # Check that the parent node is in the tree
        raise "Parent node: #{parentName} does not exist." unless @tree.has_key?(parentName)

        # Make sure we have a new bname 
        raise "Empty brick name, will not add!" if (bname.nil?)

        # Check to see if the brick already exists, if so raise an exception
        raise "Brick node: #{bname} already exists in the tree." if @tree.has_key?(bname)

        # Update @tree indexing structure -- playing around a bit here with the
        # hash as a full representation of the tree and data.
        @tree[parentName]['children'].push(bname)
        @tree[bname] = {
            'brick' => bname, 
            'parent' => parentName, 
            'budget' => budget, 
            'tags' => tags, 
            'timeWorked' => [], 
            'children' => [] 
        }
    end

    #
    #-- 
    # Method: removeSubTree
    #
    # Description:
    #   This method will do a preorder traversal of the tree and remove all of
    #   a particular subtree starting at the given brick. Once all of the tree
    #   is removed it must then remove the link in the parent's children list. 
    #
    # Input:
    #   - Name of the brick to eliminate
    #--  
    #
    def removeSubTree(bname)
        # Remove the brick from it's parent
        # Recursively remove this brick and its children 
        self.traverse_postorder(bname) do |brick|
            # Remove the brick from the hash
            self.removeBrick(brick['brick'])

            # Now remove the parent node's link to the brick
            self.removeChildLink(brick['parent'], brick['brick'])
        end
    end

    def removeBrick(bname)
        @tree.delete(bname)
    end

    # --
    # Function: removeChildLink
    #
    # Description: Take the name of the parent, find the parent, and eleminate
    # the child name link.
    #
    # NOTE: This assumes that we are taking the name of the parent and not the
    # parent node itself.
    #
    # Inputs:
    #   - parent -- name of parent
    #   - child -- name of the child
    # --
    def removeChildLink(parent, child)
        raise "Brick node: #{parent} does not exist." unless @tree.has_key?(parent)
        @tree[parent]['children'].delete(child)
    end
    
    # -- -- 
    # Function: addChildLink
    #
    # Description: Add a link to the given child in the parent
    # -- -- 
    def addChildLink(parent, child)
        raise "Brick node: #{parent} does not exist." unless @tree.has_key?(parent)
        @tree[parent]['children'].push(child)
    end

    # ---
    #
    # Function: moveWithChildren
    #
    # Description: Move the brick to the given parent including the children.
    #
    # Inputs: 
    #   - BrickName to move
    #   - Parent to move to
    # ---
    def moveWithChildren(bname, newParent)
        # Make sure the parent exists and find the parent
        raise "Brick node: #{newParent} does not exist." unless @tree.has_key?(newParent)
       
        # Make sure the child exists
        raise "Brick node: #{bname} does not exist." unless @tree.has_key?(bname)

        # Remove link to the brick from old parent
        oldParentName = self.getParent(bname)['brick']
        self.removeChildLink(oldParentName, bname)

        # Update the brick's parent 
        @tree[bname]['parent'] = newParent
        
        # Add link to the brick to the new parent 
        self.addChildLink(newParent,bname)
    end
    
    # ---
    #
    # Function: renameBrick
    #
    # Description: Rename the particular brick
    #
    # Inputs: 
    #   - new Name: Old brick name
    #   - oldName:  New brick name
    # ---
    def renameBrick(oldName, newName)
        # Raise an error if the new name is already taken
        raise "Brick node: #{newName} is already in the tree." if @tree.has_key?(newName)
       
        # Make sure the child exists
        raise "Brick node: #{oldName} does not exist." unless @tree.has_key?(oldName)

        # Remove link to the old brick name from old parent
        parentName = self.getParent(oldName)['brick']
        self.removeChildLink(parentName, oldName)
        
        # Add link to the parent for the new brick name
        self.addChildLink(parentName ,newName)
        
        #--
        # Update the brick's name
        #--
        # Add hash element to tree under new name
        @tree[newName] = @tree[oldName]
        @tree[newName]['brick'] = newName
        @tree[newName]['parent'] = parentName

        # Erase old hash element
        @tree.delete(oldName)
    end

    def getParent(bname)
        @tree[@tree[bname]["parent"]]
    end

    def getBrickFromName(bname)
        getBrick(bname, @root)
    end 

    # ---
    #
    # Function: setBrickBudget
    #
    # Description: Modify the pomodoro budget for the brick
    #
    # Inputs: 
    #   - brick: 
    #   - budget: the number of pomodoros
    # ---
    def setBrickBudget(brick, budget)
        # Make sure the brick exists and update budget
        raise "Brick node: #{brick} does not exist." unless @tree.has_key?(brick)
        @tree[brick]['budget'] = budget
    end

    # This function is deprecated and might need to be removed. An earlier data
    # structure for the brick was to use a class with pointers, such as in a
    # traditional C type data structure. Now we just use the hash.
    def getBrick(bname, bnode)
        #puts "Checking bnode: #{bnode.name} for match..."
        return bnode if (bname == bnode.name)

        bnode.children.each {|key, b|
            return getBrick(bname, b)
        }

        return false
    end

    # This function adds a brick time instance to the brick of the given name
    def recordTime(tname, bname, tstart, tend, tags)
        # Check that the parent node is in the tree
        raise "Brick node: #{bname} does not exist." unless @tree.has_key?(bname)

        # Create a time record and add it to the timeWorked array
        @tree[bname]['timeWorked'].push(BrickTimeRecord.new(tname, tstart, tend, tags))
    end

    ##----
    # Description: Get the amount of aggregate time for the given brick between
    # the given dates.
    ##----
    def brickTimeDirect(bname, dayStart, dayFinish)
        # Check that the parent node is in the tree
        raise "<bttd> Brick node: #{bname} does not exist." unless @tree.has_key?(bname)
        sum = 0
        @tree[bname]['timeWorked'].each do |btr|
            sum += btr.duration if(TimeUtils.isInDateRange(btr.endTime, dayStart, dayFinish))
        end
        sum
    end
    
    def brickBudget(bname)
        # Check that the brick exists
        raise "<bttd> Brick node: #{bname} does not exist." unless @tree.has_key?(bname)
        b = 0
        b = @tree[bname]['budget'] if !@tree[bname]['budget'].nil?
        b 
    end
    
    #def brickDayTimeDirect(bname)
        ## Check that the parent node is in the tree
        #raise "<bttd> Brick node: #{bname} does not exist." unless @tree.has_key?(bname)
        #sum = 0
        #@tree[bname]['timeWorked'].each do |btr|
            ##sum = sum + btr.duration if (TimeUtils.isToday(btr.endTime))
        #end
        #sum
    #end
    
    def brickTotalTimeDirect(bname, timeBegin=0, timeEnd=0)
        # Check that the parent node is in the tree
        raise "<bttd> Brick node: #{bname} does not exist." unless @tree.has_key?(bname)
        sum = 0
        @tree[bname]['timeWorked'].each do |btr|
            if(timeBegin == 0) 
                then sum = sum + btr.duration
            else if(btr.inRange(timeBegin,timeEnd))
                then sum = sum + btr.duration
            end
            end
        end
        sum
    end
    
    def brickWeekTimeDirect(bname)
        # Check that the parent node is in the tree
        raise "<bttd> Brick node: #{bname} does not exist." unless @tree.has_key?(bname)
        sum = 0
        @tree[bname]['timeWorked'].each do |btr|
            sum = sum + btr.duration if (TimeUtils.sinceSunday(btr.endTime))
        end
        sum
    end

    def brickDayTimeAggregate(brickName)
        raise "Brick node: #{brickName} does not exist can't calculate time." unless @tree.has_key?(brickName)
        brickAggregateTime = 0
        self.traverse_postorder(brickName) do |brick|
            brickAggregateTime += brickTimeDirect(brick['brick'], Time.now, Time.now)
        end
        brickAggregateTime
    end

    def brickTotalTimeAggregate(brickName, timeBegin=0, timeEnd=0)
        raise "Brick node: #{brickName} does not exist can't calculate time." unless @tree.has_key?(brickName)
        brickAggregateTime = 0
        self.traverse_postorder(brickName) do |brick|
            brickAggregateTime += brickTotalTimeDirect(brick['brick'], timeBegin, timeEnd)
        end
        brickAggregateTime
    end

    def brickWeeklyTimeAggregate(brickName)
        raise "Brick node: #{brickName} does not exist can't calculate time." unless @tree.has_key?(brickName)
        brickAggregateTime = 0
        self.traverse_postorder(brickName) do |brick|
            brickAggregateTime += brickWeekTimeDirect(brick['brick'])
        end
        brickAggregateTime
    end

    def brickWeeklyBudgetedAgg(brickName)
        raise "Brick node: #{brickName} does not exist can't calculate time." unless @tree.has_key?(brickName)
        budget = 0
        self.traverse_postorder(brickName) do |brick|
            budget += brickBudget(brick['brick'])
        end
        budget
    end

    # This function includes a yield, which is a closure passed from the
    # context of the caller. It also does a depth first, visit children then
    # parent traversal order.
    def traverse_postorder(brick="root", &lambdaF)
        raise "Brick node: #{brick} does not exist can't traverse." unless @tree.has_key?(brick)

        @tree[brick]['children'].each {|bc| traverse_postorder(bc, &lambdaF) }

        # We pass the brick into the closure associated with the function call
        yield(@tree[brick])
    end
    
    def traverse_preorder(brick="root", depth=0, &lambdaF)
        raise "Brick node: #{brick} does not exist can't traverse." unless @tree.has_key?(brick)

        # We pass the brick into the closure associated with the function call
        yield(@tree[brick], depth)

        @tree[brick]['children'].each {|bc| traverse_preorder(bc, depth+1, &lambdaF) }
    end

    def isBrick(b)
        @tree.has_key?(b)
    end

    def hasChild(parent,child)
        # TODO: Need an exception here for parent does not exist 
        raise 'Parent node does not exist.' unless @tree.has_key?(parent)
        @tree[parent]['children'].include?(child)
    end

    # attempts to pretty print the tree.
    def prettyPrint(name, level)
        timeInSecs = brickWeeklyTimeAggregate(name)
        @base = timeInSecs if(level == 0)
        unless(timeInSecs == 0)
            brickTimePretty = TimeUtils.timeDiffPretty(brickWeeklyTimeAggregate(name))
            frontSpaceStr = "    "*(level) + '-'
            pomos = (timeInSecs * 1.0 / 60 / @@pomo_duration).round(1)
            perc = ( (timeInSecs / @base) * 100 ).round
            timeStr = "[%s -- %02i%s -- %.1f/%i]" % [brickTimePretty, perc, '%', pomos, brickWeeklyBudgetedAgg(name)]
            nameStrWDots = "%s %s" % [name, '.' * (25 - name.length)]
            print "%s %-25s%-20s\n" % [frontSpaceStr, nameStrWDots, timeStr]
            #print "    "*(level), '- ', "#{name} [#{brickTimePretty} -- #{perc}%] \n"
        end
        @tree[name]['children'].each { |brickName|
            raise "Pretty Print: child node #{brickName} is not in the tree." unless @tree.has_key?(brickName)
            prettyPrint(brickName, level+1)
        }
    end

    def prettyPrintFullTree
        prettyPrint("root", 0)
    end

    def printSubtreeTodayTime(brickName="root", indent=0)
        traverse_preorder(brickName, indent) {|brick, depth|
            if(brickDayTimeAggregate(brick['brick']) > 0)
                timeDiffPretty = TimeUtils.timeDiffPretty(brickDayTimeAggregate(brick['brick']))
                print "    " * depth, "#{brick['brick']}: [#{timeDiffPretty}] [#{(brickDayTimeAggregate(brick['brick'])/60/@@pomo_duration).round(1)}]\n" 
            end
        }
    end

    #
    # For the given date and brick: return the array of time records
    #
    def timeRecordsForDate(brick, date)
    end


    #-----
    # Function: printByTimeRecords
    #
    # Description: 
    #   Print out the time records only per day
    #
    # Input:
    #   - Begin Time object
    #   - End Time object
    #-----
    def printByTimeRecords(timeBegin,timeEnd)
        t0 = timeBegin
        t1 = t0+24*60*60
        while t0 < timeEnd
            puts t0.strftime("[%m.%d.%y]")
            self.traverse_preorder() {|brick, depth|
                brick['timeWorked'].each{ |tr|
                    #if(tr.inRange(timeBegin,timeEnd)) 
                    if(tr.inRange(t0,t1)) 
                        print "\t", tr, " [#{brick['brick']}]\n"
                    end
                }
            }
            t0 = t1
            t1 = t1+24*60*60
        end
    end
        
    #self.traverse_postorder(brickName) do |brick|
            ##brickAggregateTime += brickTotalTimeDirect(brick['brick'], timeBegin, timeEnd)
        #end

    def printTimeRecords(dateBegin, dateEnd)
        tasks = []
        self.traverse_preorder() {|brick, depth|
            #print "    " * depth, "#{brick['brick']}:\n"
            brick['timeWorked'].each{ |tr|
                if(tr.inRange(dateBegin,dateEnd)) 
                    #print "    " * (depth+1), tr, "\n"
                    #print "    " * (depth+1), tr, "\n"
                    print "\t", tr, "--", brick['brick'],"\n"
                end
                tasks.push(tr)
            }
        }
        tasks
    end

    def getTimeRecords(dateBegin, dateEnd)
        tasks = []
        self.traverse_preorder() {|brick, depth|
            #print "    " * depth, "#{brick['brick']}:\n"
            brick['timeWorked'].each{ |tr|
                tasks.push(tr) if(tr.inRange(dateBegin,dateEnd))
            }
        }
        tasks
    end
    
    # ---------
    # Function: printSubtreeTotalTime
    #
    # Description: 
    #   Starting at the given root, do a preorder tree traversal to collect
    #   total brick work times. While going through print out the tree, given
    #   the preorder traversal. 
    # ---------
    def printSubtreeTotalTime(brickName="root", indent=0, timeBegin=0, timeEnd=0, tasks=false)
        traverse_preorder(brickName, indent) {|brick, depth|
            timeInSecs = brickTotalTimeAggregate(brick['brick'], timeBegin, timeEnd)
            unless(timeInSecs == 0)
                timeDiffPretty = TimeUtils.timeDiffPretty(timeInSecs)
                print "    " * depth, "#{brick['brick']}: [#{timeDiffPretty}] [#{timeInSecs/60/@@pomo_duration}]\n" 
                if(tasks)
                  brick['timeWorked'].each{ |tr|
                    if(tr.inRange(timeBegin,timeEnd)) 
                      print "    " * (depth+1), tr, "\n"
                    end
                  }
                end
            end
        }
    end

    #def printTasks(bri

    def printTasksByDay(brickName="root", indent=0, timeBegin=0, timeEnd=0, tasks=false)
      daystasks = []
      # for each day in range
      (DateTime.parse(timeBegin.to_s)..DateTime.parse(timeEnd.to_s)).each{|date|
        puts date.strftime('%m.%d.%y -- %a')
        daystasks = getTimeRecords(date.to_time, (date+1).to_time)
        daystasks.sort_by(&:startTime).each{|task| print "\t", task, "\n"}
      }
    end

    def printSubtree(brickName="root", indent=0)
        #traverse_preorder(brickName) {|brick|
            #print brick['brick'], "\n"
            #print ' ' * indent, "#{brick['brick']}: #{brickTotalTimeAggregate(brick['brick'])} \n"
            #indent += 1
        #}
    end
    
    def to_s
        treeStr = ""
        self.traverse_preorder() {|brick, depth|
            treeStr = treeStr + ("    " * depth) + "#{brick['brick']}:\n"
        }
        return treeStr
    end
    
end
