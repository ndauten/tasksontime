#!/usr/bin/perl
use strict;
use warnings;
use IO::Select;
use POSIX qw(strftime);
use Mail::Sendmail;
use Getopt::Long;
use DateTime;
use Scalar::Util qw(looks_like_number);
use Switch;
use Time::ParseDate;


#===============================================================================
# Support Functions 
#===============================================================================

sub usage(){
    print "\n";
    print "Usage: ./ctimer.pl [options] minutes\n\n";
    print "\t-f pomodoro_count_file (default: running_pomo_count.txt)\n";
    print "\t-b specify that this timer is a break\n";
    print "\t-r reset pomo counter.\n";
    print "\n";
    exit;
}

sub mail_notification(){
    my $now_string = shift(@_);
    print "Mailing notification of completion\n\n";
    my %mail = ( To      => 'nathan.dautenhahn@intel.com',
        From    => 'nathan.dautenhahn@intel.com',
        Message => "Pomodoro Completed!",
        Subject => "Pomodoro Completed at $now_string"
    );
    sendmail(%mail) or die $Mail::Sendmail::error;
    #print "OK. Log says:\n", $Mail::Sendmail::log;
}

sub notify_completion()
{
    my $now_string = strftime "%a %b %e %H:%M:%S %Y", localtime;

    &mail_notification($now_string);
    system("DISPLAY=:2.0 ./naughty-notify.sh");
    system("DISPLAY=:1.0 ./naughty-notify.sh");

    #foreach my $i (1 .. 190000){print("KABOOM!!!!");}
    #system('./asciiquarium');

    if(0)
    {
        if (`uname` ne "Linux"){
            &mail_notification($now_string);
            system("DISPLAY=:2.0 ./naughty-notify.sh");
            system("DISPLAY=:1.0 ./naughty-notify.sh");
        } else {    
            system('osascript ./mac_growl_notify.scpt');
            #system('say "You are done! Successful completion of Pomodoro!"');
        }
    }
}

sub printStats()
{
    system("./pomo_point_count.pl -f ../today.txt -s");
}

# Subtract from the running count of pomodoros
sub delete_pomodoros()
{
    my ($running_fn, $pomos) = @_;
    my $count = 0;
    my $trash = 0;
    if (-e $running_fn) {
        open(FH, $running_fn); ($count,$trash) = split('\n',<FH>); close(FH); 
    }
    open(FH, ">", "$running_fn")
        or die "cannot open > $running_fn: $!";
    $count -= $pomos; print FH $count; close(FH);
    print "Deleting $pomos from count\n";
    print "\n\n**** You have completed $count pomodoros ****\n\n";
}

# Add to the running count of pomodoros
sub count_pomodoro()
{
    my ($running_fn, $pomos) = @_;
    my $count = 0;
    my $trash = 0;
    if (-e $running_fn) {
        open(FH, $running_fn); ($count,$trash) = split('\n',<FH>); close(FH); 
    }
    open(FH, ">", "$running_fn")
        or die "cannot open > $running_fn: $!";
    $count += $pomos; print FH $count; close(FH);
    print "\n\n**** You have completed $count pomodoros ****\n\n";
}

# Take a time range and add pomodoros to the log and start a new one
sub count_pomodoro_from_time()
{
    my ($pomo_count_file, $beginTime, $endTime) = @_;
    my $t1 = parsedate($beginTime);
    my $t2 = parsedate($endTime);
    my $tdiff = $t2 - $t1;      # time duration in seconds
    my $full_pomos = int(($tdiff / 60) / 25);  
    my $partial_pomos = ($tdiff / 60) % 25; 
    print "\n\tAdding $full_pomos to the logfile $pomo_count_file.\n";
    print "\n\t$partial_pomos minutes extra worked\n";
    &count_pomodoro($pomo_count_file, $full_pomos);
    return 25 - $partial_pomos;
}

# Execute a timed pomodoro
sub do_pomodoro(){

    my ($s, $minutes) = @_;

    my $duration = $minutes * 60; # duration in seconds means input in minutes

    my $now_string = strftime "%a %b %e %H:%M:%S %Y", localtime;
    my $beg_time = time;
    my $end_time = $beg_time + $duration; 
    my $input = '';

    print "\nStarting timer for $minutes minutes on $now_string\n\n";
    for (;;) {
        $|++;
        my $time = time;
        last if ($time >= $end_time);
        printf("\r\t[ %02d:%02d:%02d ] -- command [p,f,v,r,q,s]? %s",
            ($end_time - $time) / (60*60),
            ($end_time - $time) / (   60) % 60,
            ($end_time - $time)           % 60,
            $input
        );
        if($s->can_read(.1)){
            chomp(my $input = <STDIN>);
            #print "Got '$foo' from STDIN\n";
            #chomp($foo = <STDIN>);
            if($input eq 'f')
            {
                print "\nExiting and counting the pomodoro\n\n";
                return 1;
            }
            if($input eq 'p'){
                $now_string = strftime "%H:%M:%S", localtime;
                print "\tPaused: $now_string ... Press enter to continue.";
                my $begin_pause = time;
                chomp($input = <STDIN>);
                $now_string = strftime "%H:%M:%S", localtime;
                print "\tResuming... $now_string\n";
                $end_time += time - $begin_pause;
            }
            if($input eq 'r')
            {
                print "Would you really like to restart the pomodoro? [y/N]: ";
                chomp(my $input = <STDIN>);
                if($input eq 'y' || $input eq 'Y')
                {
                    my $now_string = strftime "%H:%M:%S", localtime;
                    print "Restarting Pomodoro for $minutes minutes at $now_string\n";
                    my $begin_pause = time;
                    $end_time = time + $duration;
                }
            }
            if($input eq 'v'){
                print "\nExiting without counting the pomodoro\n\n";
                return 0;
            }
            if($input eq 's'){
                &printStats();
                print "\n";
            }
            if($input eq 'q'){
                print "\nExiting without counting the pomodoro\n\n";
                exit;
            }
        }
    }

    return 1;
}


sub ask_for_action_and_get_choice()
{
    print "\nWould you like to: " .
          "\n". 
          "\n\t- [a] add minutes" . 
          "\n\t- [b] take a break" . 
          "\n\t- [c] continue from the completion of the last pomodoro" .
          "\n\t- [d] delete count pomodoros" .
          "\n\t- [l] list pomodoro count" .
          "\n\t- [m] capture pomodoros from duration and start new" .
          "\n\t- [n] start a new pomodoro now".
          "\n\t- [r] reset pomodoro count" .
          "\n\t- [s] print weekly stats from today.txt" .
          "\n\t- [q] quit? ";
    print "\n\nAction: ";
    chomp(my $action = <STDIN>);
    while($action !~ /^[abcdlmnrsq]$/)
    {
        print "\nThe option $action does not exist. Please try again" .
              "[a,b,c,d,n,m,q,r,s]: ";
        chomp($action = <STDIN>);
    }
    return $action;
}

#===============================================================================
# Main script
#===============================================================================
#### OPTIONS ####
my $pomo_count_file = 'running_pomo_count.txt';
my $help = '';
my $minutes = 0;
my $break = 0;
my $reset = 0;
my $test = 0;
my $end_time = 0; 

GetOptions (
        'help' => \$help, 
        'file=s' => \$pomo_count_file, 
        'break' => \$break,
        'test' => \$test,
        'reset' => \$reset
    );

&usage if($help);
#&usage if($help || !$ARGV[0]);
#$minutes = $ARGV[0];

#-----------------------------------------------------------------------------
# If the value is not a number die. 
#-----------------------------------------------------------------------------
die "Value is not a number!\n" if ( !(&looks_like_number($minutes)) );

if($reset) {
    print "Would you really like to reset the pomodoros? [y/N] ";
    if(getc(STDIN) eq 'y') {
        system("rm -rf $pomo_count_file");
    }
}

my $s = IO::Select->new();
$s->add(\*STDIN);

while (1)
{
    my $action = &ask_for_action_and_get_choice();
    $break = 0;

    switch($action)
    {
        case 'a'
        {
            print "\n\tMinutes to add: ";
            chomp(my $tdiff_mins = <STDIN>);

            my $full_pomos = int($tdiff_mins  / 25); 
            my $partial_pomos = $tdiff_mins % 25; 
            &count_pomodoro($pomo_count_file, $full_pomos);

            $minutes = 25 - $partial_pomos;

            print "\n<<< You have $partial_pomos minutes extra worked\n";
        }
        case 'b'
        {
            $minutes = 5;
            $break = 1;
        }
        case 'l' 
        {
            my $count; my $trash;
            if (-e $pomo_count_file) {
                open(FH, $pomo_count_file); 
                ($count,$trash) = split('\n',<FH>); 
                close(FH); 
            } else {
                $count = 0;
            }
            print "\nYou have completed $count pomodoros\n";
        }
        case 'c'
        {
            if($end_time == 0)
            {
                print "Cannot continue from nothing... :)\n";
                next;
            }
            my $tdiff = time - $end_time;
            my $full_pomos = int(($tdiff / 60) / 25); 
            my $partial_pomos = ($tdiff / 60) % 25; 
            &count_pomodoro($pomo_count_file, $full_pomos);
            $minutes = 25 - $partial_pomos;
        }
        case 'd'
        {

            print "Enter the number pomos to delete: ";
            chomp(my $num_pomos = <STDIN>);
            if($num_pomos =~ /^\d+$/){
                &delete_pomodoros($pomo_count_file,$num_pomos);
            }
            else {
                print "\n\tERROR: $num_pomos is not a positive integer\n";
            }
        }
        # TODO: Add a continuous mode here to allow for free flow 
        # case 'f'
        # {
        #   start a timer counting up with the time since start of the
        #   continuous mode. Have an option to end the mode and ... but what if
        #   we've gone continous and haven't stopped when we should? How do we
        #   handle that? 
        #
        #   Can have a pause button, so effectively this works as a forward
        #   counter. It is effectively a stop watch timer approach. The main
        #   issue as already mentioned is handling that special case... I
        #   supose we could void it but .... maybe I should think of how I
        #   would handle it in real life. 
        # }
        case 'n'
        {
            print "Duration: ";
            chomp(my $duration = <STDIN>);
            if($duration eq ""){
                $minutes = 25;
            } else {
                $minutes = $duration;
            }
        }
        case 'm'
        {
            print "\n\tThe start time: ";
            chomp(my $beginTime = <STDIN>);
            print "\tThe end time: ";
            chomp(my $endTime = <STDIN>);
            $minutes = &count_pomodoro_from_time($pomo_count_file, $beginTime, $endTime);
        }
        case 'r'
        {
            print "Would you really like to reset the pomodoros? [y/N] ";
            chomp(my $answer = <STDIN>);
            if($answer eq 'y') {
                system("rm -rf $pomo_count_file");
            }
        }
        case 's'
        {
            &printStats();
        }
        case 'q'
        {
            exit();  
        }
        default:
        {
            die "That option does not exist";
        }

    }
   
    next if ($action =~ /[rlads]/);
    
    my $result = &do_pomodoro($s, $minutes);

    $end_time = time;
    
    my $now_string = strftime "%a %b %e %H:%M:%S %Y", localtime;
    # The \a is a system bell so it sounds even when I'm logged remotely
    print "\nFinished at $now_string.\a\n";

    if( $result == 1){
        &count_pomodoro($pomo_count_file, 1) if(!$break);
        &notify_completion()
    }
    else
    {
        next;
    }
}
