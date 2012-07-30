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

# Add to the running count of pomodoros
sub count_pomodoro(){
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
        printf("\r\t[ %02d:%02d:%02d ] -- command [f,p,q]? %s",
            ($end_time - $time) / (60*60),
            ($end_time - $time) / (   60) % 60,
            ($end_time - $time)           % 60,
            $input
        );
        if($s->can_read(.1)){
            chomp(my $input = <STDIN>);
            #print "Got '$foo' from STDIN\n";
            #chomp($foo = <STDIN>);
            last if($input eq 'f');
            if($input eq 'p'){
                $now_string = strftime "%H:%M:%S", localtime;
                print "\tPaused: $now_string ... Press enter to continue.";
                my $begin_pause = time;
                chomp($input = <STDIN>);
                $now_string = strftime "%H:%M:%S", localtime;
                print "\tResuming... $now_string\n";
                $end_time += time - $begin_pause;
            }
            if($input eq 'q'){
                print "\nExiting without counting the pomodoro\n\n";
                exit();
            }
        }
    }

    $now_string = strftime "%a %b %e %H:%M:%S %Y", localtime;
    # The \a is a system bell so it sounds even when I'm logged remotely
    print "\nFinished at $now_string.\a\n";
}


sub ask_for_action_and_get_choice()
{
    print "\nGreat job! Finished a pomodoro, would you like to: " .
          "\n". 
          "\n\t- [b] take a break" . 
          "\n\t- [c] continue from the completion of the last pomodoro" .
          "\n\t- [n] start a new pomodoro now".
          "\n\t- [q] quit? ";
    print "\n\t::";
    chomp(my $action = <STDIN>);
    while($action !~ /[bcnq]/)
    {
        print "\nThat option does not exist. Please try again [b,c,n,q]: ";
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

GetOptions (
        'help' => \$help, 
        'file=s' => \$pomo_count_file, 
        'break' => \$break,
        'test' => \$test,
        'reset' => \$reset
    );

&usage if($help || !$ARGV[0]);
$minutes = $ARGV[0];

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

##
# I want it to automatically caclulate the next pomodoro begin time here. 
# Outline of code: 
#
while (1)
{
    &do_pomodoro($s, $minutes);
    
    my $end_time = time;
    
    &count_pomodoro($pomo_count_file, 1) if(!$break);

    my $now_string = strftime "%a %b %e %H:%M:%S %Y", localtime;
    &mail_notification($now_string);
    system("DISPLAY=:2.0 ./naughty-notify.sh");
    system("DISPLAY=:1.0 ./naughty-notify.sh");

    my $action = &ask_for_action_and_get_choice();

    switch($action)
    {
        case 'b'
        {
            $minutes = 5;
            $break = 1;
        }
        case 'c'
        {
            my $tdiff = time - $end_time;
            my $full_pomos = int(($tdiff / 60) / 25); 
            my $partial_pomos = ($tdiff / 60) % 25; 
            &count_pomodoro($pomo_count_file, $full_pomos);
            $minutes = 25 - $partial_pomos;
            $break = 0;
        }
        case 'n'
        {
            $minutes = 25;
            $break = 0;
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
#    if( count_and_continue )
#    {
#        &count_pomodoro();
#        continue;
#    } 
#    elsif (count_and_quit)
#    {
#        &count_pomodoro();
#        last;
#    } 
#    elsif( void_and_restart )
#    {
#        continue;
#    } 
#    elsif (void_and_quit)
#    {
#        last;
#    }
}

#foreach my $i (1 .. 190000){print("KABOOM!!!!");}
#system('./asciiquarium');

my $now_string = strftime "%a %b %e %H:%M:%S %Y", localtime;
#if(!$break) 
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
