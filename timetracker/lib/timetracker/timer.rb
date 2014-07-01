#!/usr/bin/ruby
#

class Timer
    attr_reader :startTime, :endTime
    attr_writer :startTime, :endTime
    def initialize()
        @startTime = Time.now
    end

    def start
        @startTime = Time.now
    end

    def stop
        @endTime = Time.now
    end

    def timeDiffNowStr
        seconds = Time.now - @startTime
        "%02d:%02d:%02d" % [
            seconds / (60*60),
            seconds / 60 % 60,
            seconds % 60
        ]
    end

    def printTimeDiffNow
        seconds = Time.now - @startTime
        print "\n\t**** The time difference: "
        print "%02d:%02d:%02d ****\n" % [
            seconds / (60*60),
            seconds / 60 % 60,
            seconds % 60
        ]
    end

    def timeDiffSecs
        @endTime - @startTime
    end
end

module TimeUtils
    def TimeUtils.timeDiffPretty(seconds)
        "%02d:%02d:%02d" % [
            seconds / (60*60),
            seconds / 60 % 60,
            seconds % 60
        ]
    end

    # This function determines if the given time object occured since the most
    # recent sunday.
    def TimeUtils.sinceSunday(time)
        # Get a time object for right now
        now = Time.new

        # Get the date time object for the most recent sunday 12 am
        sunday = TimeUtils.getLastSundayTime

        # Do the comparison and return the result 
        #print "Comparing: Last Sunday: #{sunday}, time: #{time}, compare: #{time>sunday}\n"

        time > sunday
    end

    # Calculate the Time object for the most recent sunday by subtracting off
    # the number of days, hours, mins, and seconds in units of seconds.
    def TimeUtils.getLastSundayTime
        now = Time.new
        now - (now.wday*24*60*60) - (now.hour*60*60) - (now.min*60) - now.sec
    end

    #----------
    # Function: getBeginEnd
    #
    # Description: This function takes both a range and date option and creates
    #   the Time objects representing the start and end times of the range.
    #
    # Input: The function assumes it is getting a dates in the form of dd.mm.yy
    #    - range : a date range
    #    - day : a date day
    #---------
    #def TimeUtils.getBeginEnd(options[:r],options[:d])
    def TimeUtils.getBeginEndTimes(bDate,eDate)
        daySecs = (60*60*24)
        dateB = bDate.split('.')
        dateE = eDate.split('.')

        # create a Time object for the start date, select the 0th hour
        dateBegin = Time.new("20#{dateB[2]}", dateB[0], dateB[1])

        # We do an inclusive end date, so create an object for the end date and
        # add a day. 
        dateEnd = Time.new("20#{dateE[2]}", dateE[0], dateE[1]) + daySecs

        # return the array
        [dateBegin, dateEnd]
    end

    #
    # Note this assumes right now that we convert the day not worrying about
    # the time.
    def TimeUtils.isInDateRange(time, dayStart, dayFinish)
        d1 = Time.new(dayStart.year, dayStart.month, dayStart.day, 0, 0, 0, "-06:00")
        d2 = Time.new(dayFinish.year, dayFinish.month, dayFinish.day+1, 0, 0, 0, "-06:00")
        #puts "Requested time: #{time}: start range: #{d1}, end range: #{d2}"
        d1 < time && time < d2
    end
end
