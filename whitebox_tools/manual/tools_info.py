"""
This script is just used to automatically generate the documentation for each
of the plugin tools in the WhiteboxTools User Manual. It should be run each time new
tools are added to WhiteboxTools.exe and before a public release.
"""
from __future__ import print_function
import os
import re
import json
import sys
sys.path.append(
    '/Users/johnlindsay/Documents/programming/Whitebox/trunk/whitebox_tools/')
from whitebox_tools import WhiteboxTools

_underscorer1 = re.compile(r'(.)([A-Z][a-z]+)')
_underscorer2 = re.compile('([a-z0-9])([A-Z])')


def camel_to_snake(s):
    subbed = _underscorer1.sub(r'\1_\2', s)
    return _underscorer2.sub(r'\1_\2', subbed).lower()


wbt = WhiteboxTools()

# Set the directory containing the whitebox_tools.exe file
wbt.exe_path = r'/Users/johnlindsay/Documents/programming/Whitebox/trunk/whitebox_tools/target/release/'
# wbt.ext_path = r'../target/release/'

toolboxes = wbt.toolbox('')
tb_set = set()
for tb in toolboxes.split('\n'):
    if tb.strip():
        tb_set.add(tb.strip().split(':')[1].strip())

tb_dict = {}
for tb in sorted(tb_set):
    tb_dict[tb] = []

tools = wbt.list_tools()
for t in tools.split("\n"):
    if t.strip() and "Available Tools" not in t:
        tool = t.strip().split(":")[0]
        description = t.strip().split(":")[1].strip().rstrip('.')
        toolbox = wbt.toolbox(tool).strip()

        tool_help = wbt.tool_help(tool)
        flag = False
        example = ''
        for v in tool_help.split('\n'):
            if flag:
                example += v + "\n"
            if "Example usage:" in v:
                flag = True

        if len(example) > 65:
            a = example.split('-')
            example = ''
            count = 0
            b = 0
            for v in a:
                if v.strip():
                    if count + len(v) < 65:
                        if not v.startswith('>>'):
                            example += "-{} ".format(v.strip())
                            count += len(v) + 2
                        else:
                            example += "{} ".format(v.strip())
                            count = len(v) + 1

                    else:
                        example += "^\n-{} ".format(v.strip())
                        count = len(v) + 1
                else:
                    a[b + 1] = "-" + a[b + 1]

                b += 1

        doc_str = ""
        parameters = wbt.tool_parameters(tool)
        j = json.loads(parameters)
        param_num = 0

        tool_snaked = camel_to_snake(tool)
        if tool_snaked == "and":
            tool_snaked = "And"
        if tool_snaked == "or":
            tool_snaked = "Or"
        if tool_snaked == "not":
            tool_snaked = "Not"

        fn_def = "{}(".format(tool_snaked)
        default_params = []
        arg_append_str = ""
        # parameter_num = 1

        for p in j['parameters']:
            st = r"{}"
            st_val = '        '
            if param_num == 0:
                st_val = ''
            param_num += 1

            json_str = json.dumps(
                p, sort_keys=True, indent=2, separators=(',', ': '))

            flag_str = ""
            for v in p['flags']:
                flag_str += "{}, ".format(v.replace('--', '-\-'))
            flag_str = flag_str.rstrip(', ')
            desc = p['description'].strip().rstrip('.')
            if len(desc) > 80:
                a = desc.split(' ')
                desc = ''
                count = 0
                for v in a:
                    if count + len(v) < 80:
                        desc += "{} ".format(v)
                        count += len(v) + 1
                    else:
                        desc += "\n{}{} ".format(' ' * 21, v)
                        count = len(v) + 1

            doc_str += "{}{}{}\n".format(flag_str, ' ' * (21 - len(flag_str)),
                                         desc)

            flag = p['flags'][len(p['flags']) - 1].replace('-', '')
            if flag == "class":
                flag = "cls"

            pt = p['parameter_type']
            if 'Boolean' in pt:
                if p['default_value'] != None and p['default_value'] != 'false':
                    default_params.append(
                        "{}=True, ".format(camel_to_snake(flag)))
                else:
                    default_params.append(
                        "{}=False, ".format(camel_to_snake(flag)))

                arg_append_str += "{}if {}: args.append(\"{}\")\n".format(
                    st_val, camel_to_snake(flag), p['flags'][len(p['flags']) - 1])
            else:
                if p['default_value'] != None:
                    if p['default_value'].replace('.', '', 1).isdigit():
                        default_params.append("{}={}, ".format(
                            camel_to_snake(flag), p['default_value']))
                    else:
                        default_params.append("{}=\"{}\", ".format(
                            camel_to_snake(flag), p['default_value']))

                    arg_append_str += "{}args.append(\"{}={}\".format({}))\n".format(
                        st_val, p['flags'][len(p['flags']) - 1], st, camel_to_snake(flag))
                else:
                    if not p['optional']:
                        # if parameter_num == 1:
                        #     fn_def += "{}, ".format(camel_to_snake(flag))
                        # else:
                        fn_def += "\n    {}, ".format(camel_to_snake(flag))

                        # parameter_num += 1

                        arg_append_str += "{}args.append(\"{}='{}'\".format({}))\n".format(
                            st_val, p['flags'][len(p['flags']) - 1], st, camel_to_snake(flag))
                    else:
                        default_params.append(
                            "{}=None, ".format(camel_to_snake(flag)))
                        arg_append_str += "{}if {} is not None: args.append(\"{}='{}'\".format({}))\n".format(
                            st_val, flag, p['flags'][len(p['flags']) - 1], st, camel_to_snake(flag))

                    # arg_append_str += "{}args.append(\"{}='{}'\".format({}))\n".format(
                    #     st_val, p['flags'][len(p['flags']) - 1], st, camel_to_snake(flag))

        for d in default_params:
            # if parameter_num == 1:
            #     fn_def += d
            # else:
            fn_def += '\n    ' + d

            # parameter_num += 1

        fn_def += "\n    callback=default_callback)"

        fn = """
#### insertNumHere {}

{}.

*Parameters*:

**Flag**             **Description**
-------------------  ---------------
{}

*Python function*:
```Python
{}


```
*Command-line Interface*:
```
{}


```
""".format(tool, description, doc_str, fn_def, example)
        # print(fn)
        tb_dict[toolbox].append(fn)

f = open("/Users/johnlindsay/Documents/deleteme2.txt", 'w')
num1 = 1
num2 = 1
for key, value in sorted(tb_dict.items()):
    f.write("### 7.{} {}\n".format(num1, key.replace("/", " => ")))
    # print("* 6.{} [{}](#{})".format(num1, key.replace("/", " = "),
    #                                 key.replace("/", " = ").lower().replace(" ", "-")))
    num2 = 1
    for v in value:
        # print(v)
        f.write("{}\n".format(
            v.replace("insertNumHere", "7.{}.{}".format(num1, num2))))
        num2 += 1

    num1 += 1

f.close()
