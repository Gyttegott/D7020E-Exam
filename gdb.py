#!/usr/bin/env python
import gdb
import os
import sys
import struct
from subprocess import call
import subprocess
import glob

""" ktest file version """
version_no = 3

#debug = False
debug = True
autobuild = True

debug_file = "resource"

#klee_out_folder = 'target/x86_64-unknown-linux-gnu/debug/examples/'
klee_out_folder = 'target/x86_64-unknown-linux-gnu/release/examples/'
stm_out_folder = 'target/thumbv7em-none-eabihf/release/examples/'

file_list = []
file_index_current = -1
object_index_current = 0

tasks = []
priorities = []

task_name = ""
file_name = ""

priority = 0
first = True

# [[ Test, Task, Cyccnt, priority/ceiling]]
outputdata = []

""" Max number of events guard """
object_index_max = 100

""" Define the original working directory """
original_pwd = os.getcwd()


""" taken from KLEE """


class KTestError(Exception):
    pass


class KTest:

    @staticmethod
    def fromfile(path):
        if not os.path.exists(path):
            print("ERROR: file %s not found" % (path))
            sys.exit(1)

        f = open(path, 'rb')
        hdr = f.read(5)
        if len(hdr) != 5 or (hdr != b'KTEST' and hdr != b"BOUT\n"):
            raise KTestError('unrecognized file')
        version, = struct.unpack('>i', f.read(4))
        if version > version_no:
            raise KTestError('unrecognized version')
        numArgs, = struct.unpack('>i', f.read(4))
        args = []
        for i in range(numArgs):
            size, = struct.unpack('>i', f.read(4))
            args.append(str(f.read(size).decode(encoding='ascii')))

        if version >= 2:
            symArgvs, = struct.unpack('>i', f.read(4))
            symArgvLen, = struct.unpack('>i', f.read(4))
        else:
            symArgvs = 0
            symArgvLen = 0

        numObjects, = struct.unpack('>i', f.read(4))
        objects = []
        for i in range(numObjects):
            size, = struct.unpack('>i', f.read(4))
            name = f.read(size)
            size, = struct.unpack('>i', f.read(4))
            bytes = f.read(size)
            objects.append((name, bytes))

        # Create an instance
        b = KTest(version, args, symArgvs, symArgvLen, objects)
        # Augment with extra filename field
        b.filename = path
        return b

    def __init__(self, version, args, symArgvs, symArgvLen, objects):
        self.version = version
        self.symArgvs = symArgvs
        self.symArgvLen = symArgvLen
        self.args = args
        self.objects = objects

        # add a field that represents the name of the program used to
        # generate this .ktest file:
        program_full_path = self.args[0]
        program_name = os.path.basename(program_full_path)
        # sometimes program names end in .bc, so strip them
        if program_name.endswith('.bc'):
            program_name = program_name[:-3]
        self.programName = program_name

# Event handling

# Ugly hack to avoid race condtitons in the python gdb API


class Executor:
    def __init__(self, cmd):
        self.__cmd = cmd

    def __call__(self):
        gdb.execute(self.__cmd)


"""
Every time a breakpoint is hit this function is executed
"""


def stop_event(evt):
    global outputdata
    global task_name
    global priority
    global file_name

    if debug:
        print("Debug: stop event in file {}".format(file_name))
        print("Debug: evt %r" % evt)

    imm = gdb_bkpt_read()
    print(" imm = {}".format(imm))

    if imm == 0:
        print("Ordinary breakpoint, exiting!")
        sys.exit(1)

    elif imm == 1 or imm == 2:
        try:
            ceiling = int(gdb.parse_and_eval(
                "ceiling").cast(gdb.lookup_type('u8')))
        except:
            print("No ceiling found, exciting!")
            sys.exit(1)

        if imm == 1:
            action = "Enter"
        elif imm == 2:
            action = "Exit"

        if debug:
            print("Debug: Append action {} at cycle {}".format(
                action, gdb_cyccnt_read()))

        outputdata.append(
            [file_name, task_name, gdb_cyccnt_read(), ceiling, action])

        gdb.post_event(Executor("continue"))

    elif imm == 3:
        if debug:
            print("Debug: found finish bkpt_3 at cycle {}".format(gdb_cyccnt_read()))

        gdb.post_event(Executor("si"))

    elif imm == 4:
        if debug:
            print("Debug: found finish bkpt_4 at cycle {}".format(gdb_cyccnt_read()))

        gdb.post_event(posted_event_init)

    else:
        print("Unexpected stop event, exiting")
        sys.exit(1)


""" Loads each defined task """


def posted_event_init():
    if debug:
        print("\nDebug: Entering posted_event_init")

    global tasks
    global task_name
    global file_name
    global file_index_current
    global file_list
    global outputdata
    global priority
    global priorities

    if file_index_current < 0:
        if debug:
            print("Debug: Skipped first measurement")

    else:
        if debug:
            print("Debug: Append Finish action at cycle {}".format(gdb_cyccnt_read()))

        outputdata.append(
            [file_name, task_name, gdb_cyccnt_read(), priority, "Finish"])

    """ loop to skip to next task *omitting the dummy* """
    while True:
        file_index_current += 1
        if file_index_current == len(file_list):
            """ finished """
            break

        task_to_test = ktest_setdata(file_index_current)
        if 0 <= task_to_test < len(tasks):
            """ next """
            break

    if file_index_current < len(file_list):
        """ Load the variable data """

        if debug:
            print("Debug: Task number to test {}".format(task_to_test))

        """
        Before the call to the next task, reset the cycle counter
        """
        gdb_cyccnt_reset()

        file_name = file_list[file_index_current].split('/')[-1]
        task_name = tasks[task_to_test]
        priority = priorities[task_to_test]

        outputdata.append([file_name, task_name,
                           gdb_cyccnt_read(), priority, "Start"])

        print('Task to call: %s \n' % (
            tasks[task_to_test] + "()"))
        gdb.execute('call %s' % "stub_" +
                    tasks[task_to_test] + "()")

    else:
        """ here we are done, call your analysis here """
        offset = 1
        print("\nFinished all ktest files!\n")
        print("Claims:")
        for index, obj in enumerate(outputdata):
            if obj[4] == "Exit":
                claim_time = (obj[2] -
                              outputdata[index - (offset)][2])
                print("%s Claim time: %s" % (obj, claim_time))
                offset += 2
            elif obj[4] == "Finish" and not obj[2] == 0:
                offset = 1
                tot_time = obj[2]
                print("%s Total time: %s" % (obj, tot_time))
            else:
                print("%s" % (obj))

        # gdb.execute("quit")


def trimZeros(str):
    for i in range(len(str))[::-1]:
        if str[i] != '\x00':
            return str[:i + 1]

    return ''


def ktest_setdata(file_index):
    """
    Substitute every variable found in ktest-file
    """
    global file_list
    global debug

    print("[[[[[[[[[[[[[[[[ ktest_setdata {}".format(file_index))
    b = KTest.fromfile(file_list[file_index])
    print("[[[[[[[[[[[[[[[[[[here")
    if debug:
        # print('ktest filename : %r' % filename)
        gdb.write('ktest file: %r \n' % file_list[file_index])
        # print('args       : %r' % b.args)
        # print('num objects: %r' % len(b.objects))
    for i, (name, data) in enumerate(b.objects):
        str = trimZeros(data)

        """ If Name is "task", skip it """
        if name.decode('UTF-8') == "task":
            if debug:
                print('object %4d: name: %r' % (i, name))
                print('object %4d: size: %r' % (i, len(data)))
            # print(struct.unpack('i', str).repr())
            # task_to_test = struct.unpack('i', str)[0]
            # print("str: ", str)
            # print("str: ", str[0])
            task_to_test = struct.unpack('i', str)[0]
            # task_to_test = int(str[0])
            if debug:
                print("Task to test:", task_to_test)
        else:
            if debug:
                print('object %4d: name: %r' % (i, name))
                print('object %4d: size: %r' % (i, len(data)))
                print(str)
            # if opts.writeInts and len(data) == 4:
            obj_data = struct.unpack('i', str)[0]
            if debug:
                print('object %4d: data: %r' %
                      (i, obj_data))
            # gdb.execute('whatis %r' % name.decode('UTF-8'))
            # gdb.execute('whatis %r' % obj_data)
            gdb.execute('set variable %s = %r' %
                        (name.decode('UTF-8'), obj_data))
            # gdb.write('Variable %s is:' % name.decode('UTF-8'))
            # gdb.execute('print %s' % name.decode('UTF-8'))
            # else:
            # print('object %4d: data: %r' % (i, str))
    print("---------------------------------------- ktest_set_end")

    if debug:
        print("Done with setdata")
    return task_to_test


def ktest_iterate():
    """ Get the list of folders in current directory, sort and then grab the
        last one.
    """
    global debug
    global autobuild

    curdir = os.getcwd()
    if debug:
        print("Current directory {}".format(curdir))

    rustoutputfolder = curdir + "/" + klee_out_folder
    try:
        os.chdir(rustoutputfolder)
    except IOError:
        print(rustoutputfolder + "not found. Need to run\n")
        print("xargo build --example " + example_name + " --features" +
              " klee_mode --target x86_64-unknown-linux-gnu ")
        print("\nand docker run --rm --user (id -u):(id -g)" +
              "-v $PWD" + "/" + klee_out_folder + ":/mnt" +
              "-w /mnt -it afoht/llvm-klee-4 /bin/bash ")
        if autobuild:
            xargo_run("klee")
            klee_run()
        else:
            print("Run the above commands before proceeding")
            sys.exit(1)

    if os.listdir(rustoutputfolder) == []:
        """
        The folder is empty, generate some files
        """
        xargo_run("klee")
        klee_run()

    dirlist = next(os.walk("."))[1]
    dirlist.sort()
    if debug:
        print(dirlist)

    if not dirlist:
        print("No KLEE output, need to run KLEE")
        print("Running klee...")
        klee_run()

    """ Ran KLEE, need to update the dirlist """
    dirlist = next(os.walk("."))[1]
    dirlist.sort()
    try:
        directory = dirlist[-1]
    except IOError:
        print("No KLEE output, need to run KLEE")
        print("Running klee...")
        klee_run()

    print("Using ktest-files from directory:\n" + rustoutputfolder + directory)

    """ Iterate over all files ending with ktest in the "klee-last" folder """
    for filename in os.listdir(directory):
        if filename.endswith(".ktest"):
            file_list.append(os.path.join(rustoutputfolder + directory,
                                          filename))
        else:
            continue

    file_list.sort()
    """ Return to the old path """
    os.chdir(curdir)
    return file_list


def tasklist_get():
    """ Parse the automatically generated tasklist
    """

    if debug:
        print(os.getcwd())
    with open('klee/tasks.txt') as fin:
        for line in fin:
                # print(line)
            if not line == "// autogenerated file\n":
                return [x.strip().strip("[]\"").split(' ') for x in line.split(',')]


""" Run xargo for building """


def xargo_run(mode):

    if "klee" in mode:
        xargo_cmd = ("xargo build --example " + example_name + " --features " +
                     "klee_mode --target x86_64-unknown-linux-gnu ")
    elif "stm" in mode:
        xargo_cmd = ("xargo build --release --example " + example_name +
                     " --features " +
                     "wcet_bkpt --target thumbv7em-none-eabihf")
    else:
        print("Provide either 'klee' or 'stm' as mode")
        sys.exit(1)

    call(xargo_cmd, shell=True)


""" Stub for running KLEE on the LLVM IR """


def klee_run():
    global debug
    global original_pwd

    PWD = original_pwd

    user_id = subprocess.check_output(['id', '-u']).decode()
    group_id = subprocess.check_output(['id', '-g']).decode()

    bc_file = (glob.glob(PWD + "/" +
                         klee_out_folder +
                         '*.bc', recursive=False))[-1].split('/')[-1].strip('\'')
    if debug:
        print(PWD + "/" + klee_out_folder)
        print(bc_file)

    klee_cmd = ("docker run --rm --user " +
                user_id[:-1] + ":" + group_id[:-1] +
                " -v '"
                + PWD + "/"
                + klee_out_folder + "':'/mnt'" +
                " -w /mnt -it afoht/llvm-klee-4 " +
                "/bin/bash -c 'klee %s'" % bc_file)
    if debug:
        print(klee_cmd)
    call(klee_cmd, shell=True)


def gdb_cyccnt_enable():
    # Enable cyccnt
    gdb.execute("mon mww 0xe0001000 1")


def gdb_cyccnt_disable():
    # Disble cyccnt
    gdb.execute("mon mww 0xe0001000 0")


def gdb_cyccnt_reset():
    # Reset cycle counter to 0
    gdb.execute("mon mww 0xe0001004 0")


def gdb_cyccnt_read():
    # Read cycle counter
    return int(gdb.execute("mon mdw 0xe0001004", False, True).strip(
        '\n').strip('0xe000012004:').strip(',').strip(), 16)


def gdb_cyccnt_write(num):
    # Write to cycle counter
    gdb.execute('mon mww 0xe0001004 %r' % num)


def gdb_bkpt_read():
    # Read imm field of the current bkpt
    try:
        return int(gdb.execute("x/i $pc", False, True).split("bkpt")[1].strip("\t").strip("\n"), 0)
    except:
        if debug:
            print("Debug: It is not a bkpt so return 4")
        return 4


print("\n\n\nStarting script")

"""Used for making GDB scriptable"""
gdb.execute("set confirm off")
gdb.execute("set pagination off")
gdb.execute("set verbose off")
gdb.execute("set height 0")

"""
Setup GDB for remote debugging
"""
gdb.execute("target remote :3333")
gdb.execute("monitor arm semihosting enable")

"""
Check if the user passed a file to use as the source.

If a file is given, use this as the example_name
"""
if gdb.progspaces()[0].filename:
    """ A filename was given on the gdb command line """
    example_name = gdb.progspaces()[0].filename.split('/')[-1]
    print("The resource used for debugging: %s" % example_name)
    try:
        os.path.exists(gdb.progspaces()[0].filename)
    except IOError:
        """ Compiles the given example """
        xargo_run("stm")
        xargo_run("klee")
else:
    example_name = debug_file
    print("Defaulting to example '%s' for debugging." % example_name)
    try:
        if example_name not in os.listdir(stm_out_folder):
            """ Compiles the default example """
            xargo_run("stm")
            xargo_run("klee")
    except IOError:
        """ Compiles the default example """
        xargo_run("stm")
        xargo_run("klee")

""" Tell GDB to load the file """
gdb.execute("file %s" % (stm_out_folder + example_name))
gdb.execute("load %s" % (stm_out_folder + example_name))

""" Tell gdb-dashboard to hide """
# gdb.execute("dashboard -enabled off")
# gdb.execute("dashboard -output /dev/null")

""" Enable the cycle counter """
gdb_cyccnt_enable()
gdb_cyccnt_reset()

""" Save all ktest files into an array """
file_list = ktest_iterate()

""" Get all the tasks to jump to """
task_list = tasklist_get()

if debug:
    print("Debug: file_list {}".format(file_list))
    print("Debug: task_list {}".format(task_list))

""" Split into tasks and priorities """
for x in task_list:
    priorities.append(x.pop())
    tasks.append(x.pop())

print("Available tasks:")
for t in tasks:
    print(t)

print("At priorities:")
for t in priorities:
    print(t)


""" Subscribe stop_event_ignore to Breakpoint notifications """
gdb.events.stop.connect(stop_event)

""" 
    continue until bkpt 3, 
    this will pick the next task (through a posted_event_init event) 
"""
gdb.execute("continue")


# Home exam, response time analysis
#
# 1. run the example and study the output
# it generates `output data`, a list of list, something like:
# Claims:
# ['', '', 56, 0, 'Finish'] Total time: 56
# ['test000002.ktest', 'EXTI2', 0, '3', 'Start']
# ['test000002.ktest', 'EXTI2', 11, '3', 'Finish'] Total time: 11
# ['test000003.ktest', 'EXTI2', 0, '3', 'Start']
# ['test000003.ktest', 'EXTI2', 11, '3', 'Finish'] Total time: 11
# ['test000004.ktest', 'EXTI3', 0, '2', 'Start']
# ['test000004.ktest', 'EXTI3', 8, '2', 'Finish'] Total time: 8
# ['test000005.ktest', 'EXTI1', 0, '1', 'Start']
# ['test000005.ktest', 'EXTI1', 15, 2, 'Enter']
# ['test000005.ktest', 'EXTI1', 19, 3, 'Enter']
# ['test000005.ktest', 'EXTI1', 28, 3, 'Exit'] Claim time: 9
# ['test000005.ktest', 'EXTI1', 29, 2, 'Exit'] Claim time: 14
# ['test000005.ktest', 'EXTI1', 36, '1', 'Finish'] Total time: 36...
#
# first entry
# ['test000001.ktest', ....
# is our bogus test, not of interest for the analysis)
#
# next entries:
#['test000002.ktest', 'EXTI2', 0, 3, 'Start']
#['test000002.ktest', 'EXTI2', 11, 3, 'Finish'] Total time: 11
# amounts to the first real task
# broken down, the first measurement
# -'test000002.ktest'       the ktest file
# -'EXTI2'                  the task
# -'0'                      the time stamp (start from zero)
# -'3'                      the threshold (priority 3)
# -'Start'                  the 'Start' event
#
# broken down, the second measurement
# -'test000002.ktest'       the ktest file
# -'EXTI2'                  the task
# -'11'                     the time stamp (start from zero)
# -'3'                      the threshold (priority 3)
# -'Finish'                 the 'Start' event
#
# followed by
# Total time: 11
#
# let us look at the following measurements
#
# 'test000003.ktest'
# recall from the lab that we had two cases for EXTI2
# both with the same result
#
# 'test000004.ktest'
# recall from the lab that we had a singel test for EXTI3
#
# and finally
#
# 'test000005.ktest' and on ... for EXTI1
# here at prio 1, and after 15 cycles claim X, raising threshold to 2
# after 19 cycles we clam Y, raising treshold to 3
# after 28 cycles we exit the Y claim, threshold 3 *before unlock Y*
# after 29 cycles we exit the X claim, threshold 2 *before unlock X*
# and finally we finish at 36 clock cycles
#
# Reecall we had some 13, 9, 39 in the lab, this is due to details
# of the gdb integration regarding the return behavior.
